# Chặn mọi kết nối ra ngoài của các tiến trình Mori (ADR-0010, Phase 6). Loopback (127.0.0.1) không bị
# Windows Firewall lọc nên Mori vẫn chạy bình thường. Cần quyền admin:
#   Start-Process powershell -Verb RunAs -ArgumentList '-ExecutionPolicy Bypass -File scripts\firewall.ps1'
# Gỡ:  Get-NetFirewallRule -Direction Outbound | ? DisplayName -like "Mori*" | Remove-NetFirewallRule
# Tên luật/nhóm dùng ASCII: PowerShell 5.1 đọc file UTF-8 không BOM sai dấu tiếng Việt.
param([string]$Data = "E:\Mori Data", [string]$Repo = (Split-Path $PSScriptRoot -Parent))
$group = "Mori - block outbound"
$programs = @(
    (Get-ChildItem "$Data\runtime\webview2" -Recurse -Filter msedgewebview2.exe -ErrorAction SilentlyContinue).FullName
    "$Data\runtime\llama.cpp\b12284-vulkan\llama-server.exe"
    "$Data\runtime\llama.cpp\b12284-cpu\llama-server.exe"
    "$Data\runtime\whisper.cpp\v1.9.5-vulkan\whisper-server.exe"
    "$Data\runtime\python\tts-venv\Scripts\python.exe"
    "$Data\runtime\python\interpreters\cpython-3.12-windows-x86_64-none\python.exe"
    "$Repo\src-tauri\target\debug\mori.exe"
    "$Repo\src-tauri\target\release\mori.exe"
) | Where-Object { $_ }
Get-NetFirewallRule -Direction Outbound | Where-Object { $_.DisplayName -like "Mori*" } | Remove-NetFirewallRule
foreach ($p in $programs) {
    New-NetFirewallRule -Group $group -DisplayName "Mori block outbound: $(Split-Path $p -Leaf)" `
        -Direction Outbound -Action Block -Program $p -Profile Any | Out-Null
    "blocked: $p"
}
"Done: $($programs.Count) rules in group '$group'"
