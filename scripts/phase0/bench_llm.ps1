# Benchmark LLM bằng llama.cpp. Không tải gì - runtime và model phải có sẵn.
#   -LlamaDir : thư mục build llama.cpp (chứa llama-bench.exe, llama-server.exe). Mỗi backend một build riêng.
#   -Backend  : cpu (-ngl 0) | vulkan | sycl (-ngl 99)
#   -Server   : thêm đo TTFT lạnh/ấm qua llama-server (prompt cache)
# Ví dụ:
#   powershell -ExecutionPolicy Bypass -File scripts/phase0/bench_llm.ps1 -LlamaDir "E:\Mori Data\runtime\llama.cpp\b1234-vulkan" -Model "E:\Mori Data\models\llm\Qwen3-4B-Q4_K_M.gguf" -Backend vulkan -Server
param(
    [Parameter(Mandatory)][string]$LlamaDir,
    [Parameter(Mandatory)][string]$Model,
    [Parameter(Mandatory)][ValidateSet('cpu', 'vulkan', 'sycl')][string]$Backend,
    [switch]$Server,
    [int]$Ctx = 8192
)
$ErrorActionPreference = 'Stop'
$ngl = if ($Backend -eq 'cpu') { 0 } else { 99 }
$root = Split-Path (Split-Path $PSScriptRoot)
$out = Join-Path $PSScriptRoot 'out'
New-Item -ItemType Directory -Force $out | Out-Null
$name = [IO.Path]::GetFileNameWithoutExtension($Model)
$log = Join-Path $out ("llm_{0}_{1}_{2:yyyyMMdd_HHmm}.txt" -f $Backend, $name, (Get-Date))

foreach ($f in $Model, (Join-Path $LlamaDir 'llama-bench.exe')) {
    if (-not (Test-Path $f)) { throw "Khong tim thay: $f" }
}

"== llama-bench | $Backend | ngl=$ngl | $name" | Tee-Object -FilePath $log
# pp512 ~ toc do xu ly prompt (quyet dinh TTFT), tg128 ~ toc do sinh. 3 lan lap.
& (Join-Path $LlamaDir 'llama-bench.exe') -m $Model -p 512 -n 128 -r 3 -ngl $ngl -o md 2>&1 |
    Tee-Object -FilePath $log -Append

if (-not $Server) { "`nDa luu: $log"; return }

# ---- TTFT qua llama-server: cung system prompt, 2 cau hoi khac nhau ----
# Lan 1 (lanh): xu ly toan bo prompt. Lan 2 (am): chi xu ly phan moi neu prompt cache hoat dong.
$port = Get-Random -Minimum 20000 -Maximum 40000
$srv = Start-Process -FilePath (Join-Path $LlamaDir 'llama-server.exe') -PassThru -WindowStyle Hidden `
    -ArgumentList @('-m', "`"$Model`"", '--host', '127.0.0.1', '--port', $port, '-c', $Ctx, '-ngl', $ngl, '--jinja')
try {
    $base = "http://127.0.0.1:$port"
    $deadline = (Get-Date).AddSeconds(180)
    while ($true) {
        try { if ((Invoke-WebRequest "$base/health" -UseBasicParsing -TimeoutSec 2).StatusCode -eq 200) { break } } catch {}
        if ($srv.HasExited) { throw "llama-server thoat som (exit $($srv.ExitCode))" }
        if ((Get-Date) -gt $deadline) { throw 'llama-server khong san sang sau 180 s' }
        Start-Sleep -Milliseconds 500
    }
    $system = Get-Content (Join-Path $root 'tests/fixtures/system_prompt_sample.md') -Raw -Encoding UTF8
    "`n== TTFT qua llama-server (ctx=$Ctx)" | Tee-Object -FilePath $log -Append
    $questions = 'Mori oi, giai thich giup minh closure trong JavaScript la gi?', 'Con async/await thi khac Promise o cho nao?'
    & { foreach ($i in 1..$questions.Count) {
        $q = $questions[$i - 1]
        $body = @{
            messages             = @(@{ role = 'system'; content = $system }, @{ role = 'user'; content = $q })
            max_tokens           = 16
            temperature          = 0
            cache_prompt         = $true
            chat_template_kwargs = @{ enable_thinking = $false }
        } | ConvertTo-Json -Depth 5
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $r = Invoke-RestMethod "$base/v1/chat/completions" -Method Post -ContentType 'application/json; charset=utf-8' `
            -Body ([Text.Encoding]::UTF8.GetBytes($body))
        $sw.Stop()
        $t = $r.timings
        $label = if ($i -eq 1) { 'lanh' } else { 'am' }
        if ($t) {
            "lan {0} ({1}): prompt_n={2} prompt_ms={3:N0} -> TTFT~{4:N2}s | gen {5:N1} tok/s | tong {6:N2}s" -f `
                $i, $label, $t.prompt_n, $t.prompt_ms, ($t.prompt_ms / 1000), $t.predicted_per_second, $sw.Elapsed.TotalSeconds
        } else {
            "lan {0} ({1}): khong co 'timings' trong response | tong {2:N2}s" -f $i, $label, $sw.Elapsed.TotalSeconds
        }
    } } | Tee-Object -FilePath $log -Append
    $mem = (Get-Process -Id $srv.Id).PeakWorkingSet64 / 1GB
    "RAM dinh llama-server: {0:N2} GB" -f $mem | Tee-Object -FilePath $log -Append
}
finally {
    if (-not $srv.HasExited) { Stop-Process -Id $srv.Id -Force }
}
"`nDa luu: $log"
