# Benchmark STT bằng whisper-cli trên bộ câu trong tests/fixtures/stt_manifest.csv.
# Audio: WAV 16 kHz mono 16-bit, đặt trong tests/fixtures/audio/ (gitignored).
# latency_s = total - load (mô phỏng whisper-server đã nạp sẵn model); load_s ghi riêng.
# Ví dụ:
#   powershell -ExecutionPolicy Bypass -File scripts/phase0/bench_stt.ps1 -Model "E:\Mori Data\models\stt\ggml-large-v3-turbo-q5_0.bin"
param(
    [string]$WhisperCli = 'E:\Mori Data\runtime\whisper.cpp\v1.9.5-vulkan\whisper-cli.exe',
    [Parameter(Mandatory)][string]$Model,
    [int]$Threads = 8,
    [string]$Language = 'vi',
    [string]$Manifest,   # mac dinh tests/fixtures/stt_manifest.csv; cot 'file' tuong doi theo -AudioDir
    [string]$AudioDir,
    [string[]]$ExtraArgs = @()   # vd: -ExtraArgs '-ac','768','-fa'
)
$ErrorActionPreference = 'Stop'
# powershell -File gop mang thanh mot chuoi "-ac,768,-fa" -> tach lai
$ExtraArgs = @($ExtraArgs | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
$root = Split-Path (Split-Path $PSScriptRoot)
$manifest = if ($Manifest) { $Manifest } else { Join-Path $root 'tests/fixtures/stt_manifest.csv' }
$audioDir = if ($AudioDir) { $AudioDir } else { Join-Path $root 'tests/fixtures/audio' }
$out = Join-Path $PSScriptRoot 'out'
$tmp = Join-Path $out 'stt_tmp'
New-Item -ItemType Directory -Force $tmp | Out-Null
$name = [IO.Path]::GetFileNameWithoutExtension($Model)
$csv = Join-Path $out ("stt_{0}{1}_{2:yyyyMMdd_HHmm}.csv" -f $name, (($ExtraArgs -join '') -replace '[^\w]', '_'), (Get-Date))

foreach ($f in $WhisperCli, $Model, $manifest) { if (-not (Test-Path $f)) { throw "Khong tim thay: $f" } }

function Get-Ms([string]$text, [string]$label) {
    if ($text -match "$label time\s*=\s*([\d.]+)\s*ms") { [double]$Matches[1] } else { [double]::NaN }
}

$rows = Import-Csv $manifest -Encoding UTF8 | Where-Object { $_.file -and $_.ref -and -not $_.file.StartsWith('#') }
$results = foreach ($row in $rows) {
    $wav = Join-Path $audioDir $row.file
    if (-not (Test-Path $wav)) { Write-Warning "Bo qua (thieu file): $($row.file)"; continue }
    $so = Join-Path $tmp 'stdout.txt'; $se = Join-Path $tmp 'stderr.txt'
    $p = Start-Process -FilePath $WhisperCli -Wait -PassThru -NoNewWindow `
        -RedirectStandardOutput $so -RedirectStandardError $se `
        -ArgumentList (@('-m', "`"$Model`"", '-f', "`"$wav`"", '-l', $Language, '-nt', '-t', $Threads) + $ExtraArgs)
    if ($p.ExitCode -ne 0) { Write-Warning "whisper-cli loi ($($p.ExitCode)): $($row.file)"; continue }
    $err = Get-Content $se -Raw
    $hyp = ((Get-Content $so -Encoding UTF8) -join ' ').Trim()
    $load = Get-Ms $err 'load'; $total = Get-Ms $err 'total'
    if ([double]::IsNaN($total) -or -not $hyp) {
        Write-Warning "Khong co ket qua/timing: $($row.file) | $(($err -split "`n" | Select-Object -Last 3) -join ' ')"; continue
    }
    Write-Host ("{0}: {1:N2}s | {2}" -f $row.file, (($total - $load) / 1000), $hyp)
    [pscustomobject]@{
        file = $row.file; category = $row.category; ref = $row.ref; hyp = $hyp
        latency_s = [math]::Round(($total - $load) / 1000, 3); load_s = [math]::Round($load / 1000, 3)
    }
}
if (-not $results) { throw "Khong co cau nao chay duoc. Kiem tra $audioDir va $manifest." }
$results | Export-Csv $csv -NoTypeInformation -Encoding UTF8
Remove-Item $tmp -Recurse -Force

"`n== $name $($ExtraArgs -join ' ') | $(@($results).Count) cau | build: $(Split-Path (Split-Path $WhisperCli) -Leaf)"
python -I (Join-Path $PSScriptRoot 'wer.py') --csv $csv
"Chi tiet tung cau: $csv"
