# Chặn mọi kết nối ra ngoài của các tiến trình Mori (ADR-0010, Phase 6). Loopback (127.0.0.1) không bị
# Windows Firewall lọc nên Mori vẫn chạy bình thường. Cần quyền admin:
#   Start-Process powershell -Verb RunAs -ArgumentList '-ExecutionPolicy Bypass -File scripts\firewall.ps1'
# Gỡ:  Get-NetFirewallRule -Direction Outbound | ? DisplayName -like "Mori*" | Remove-NetFirewallRule
# Tên luật/nhóm dùng ASCII: PowerShell 5.1 đọc file UTF-8 không BOM sai dấu tiếng Việt.
param([string]$Data = "E:\Mori Data", [string]$Repo = (Split-Path $PSScriptRoot -Parent))
$group = "Mori - block outbound"
# Mọi exe dưới runtime (llama/whisper/python/WebView2 + các helper của WebView2 — audit Phase 6 M2) + mori.exe.
$programs = @(
    (Get-ChildItem "$Data\runtime" -Recurse -Filter *.exe -ErrorAction SilentlyContinue).FullName
    "$Repo\src-tauri\target\debug\mori.exe"
    "$Repo\src-tauri\target\release\mori.exe"
) | Where-Object { $_ }
Get-NetFirewallProfile | Where-Object { -not $_.Enabled } | ForEach-Object { "WARN: firewall profile '$($_.Name)' is OFF - rules have no effect" }
Get-NetFirewallRule -Direction Outbound | Where-Object { $_.DisplayName -like "Mori*" } | Remove-NetFirewallRule
foreach ($p in $programs) {
    New-NetFirewallRule -Group $group -DisplayName "Mori block outbound: $(Split-Path $p -Leaf)" `
        -Direction Outbound -Action Block -Program $p -Profile Any | Out-Null
    if (Test-Path $p) { "blocked: $p" } else { "WARN missing (rule added anyway): $p" }
}
"Done: $($programs.Count) rules in group '$group'"
