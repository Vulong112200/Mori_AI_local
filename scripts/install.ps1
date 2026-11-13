# Install Mori (release build) for the current user: copy mori.exe into the data root (covered by ACL + firewall),
# create Start Menu + Startup shortcuts (run at Windows sign-in). No admin needed except the firewall step
# (UAC prompt once, only when the new exe path has no block rule yet - ADR-0010).
#   npm run tauri build -- --no-bundle
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1            # install / update
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -NoStartup # no auto start
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -Uninstall # remove app + shortcuts (keeps data)
# Disable auto start later: Task Manager > Startup apps > Mori, or delete the Startup shortcut.
# Taskbar: Windows 11 does not allow programs to pin themselves -> run Mori, right-click its taskbar icon > Pin.
param([string]$Data = "E:\Mori Data", [switch]$NoStartup, [switch]$Uninstall)
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$dir = Join-Path $Data "runtime\app"
$exe = Join-Path $dir "mori.exe"
$startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) "Mori.lnk"
$startup = Join-Path ([Environment]::GetFolderPath('Startup')) "Mori.lnk"

if (Get-Process mori -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe }) {
    throw "Mori is running from $exe - close it first."
}
if ($Uninstall) {
    Remove-Item $startMenu, $startup -ErrorAction SilentlyContinue
    Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
    "Removed app + shortcuts. Data in $Data is kept."
    return
}

$src = Join-Path $repo "src-tauri\target\release\mori.exe"
if (-not (Test-Path $src)) { throw "Missing $src - run: npm run tauri build -- --no-bundle" }
New-Item -ItemType Directory -Force $dir | Out-Null
Copy-Item $src $exe -Force
"installed: $exe"

$shell = New-Object -ComObject WScript.Shell
function New-Link([string]$path) {
    $l = $shell.CreateShortcut($path)
    $l.TargetPath = $exe
    $l.WorkingDirectory = $dir
    $l.IconLocation = "$exe,0"
    $l.Description = "Mori - local AI companion"
    $l.Save()
    "shortcut: $path"
}
New-Link $startMenu
if ($NoStartup) { Remove-Item $startup -ErrorAction SilentlyContinue; "auto start: off" } else { New-Link $startup; "auto start: on" }

# Firewall: block outbound for the new exe path (loopback is not filtered, Mori still works).
$blocked = Get-NetFirewallRule -Direction Outbound -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -like "Mori*" } | Get-NetFirewallApplicationFilter |
    Where-Object { $_.Program -eq $exe }
if ($blocked) {
    "firewall: already blocked"
} else {
    "firewall: no rule for $exe -> running firewall.ps1 as admin (accept the UAC prompt)"
    Start-Process powershell -Verb RunAs -Wait -ArgumentList "-ExecutionPolicy Bypass -File `"$PSScriptRoot\firewall.ps1`""
}
