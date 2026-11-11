# Thu thập thông tin phần cứng cho docs/HARDWARE_REPORT.md. CHỈ ĐỌC - không thay đổi hệ thống.
# Chạy: powershell -ExecutionPolicy Bypass -File scripts/phase0/hw_inventory.ps1
$ErrorActionPreference = 'Continue'
$out = Join-Path $PSScriptRoot 'out'
New-Item -ItemType Directory -Force $out | Out-Null
$file = Join-Path $out ("hw_inventory_{0:yyyyMMdd_HHmm}.txt" -f (Get-Date))

& {
    "== Ngay do: $(Get-Date -Format 'yyyy-MM-dd HH:mm')"

    $os = Get-CimInstance Win32_OperatingSystem
    "`n== OS"
    "{0} build {1}" -f $os.Caption, $os.BuildNumber
    "RAM tong: {0:N1} GB | trong: {1:N1} GB" -f ($os.TotalVisibleMemorySize / 1MB), ($os.FreePhysicalMemory / 1MB)
    "Commit limit: {0:N1} GB | commit trong: {1:N1} GB" -f ($os.TotalVirtualMemorySize / 1MB), ($os.FreeVirtualMemory / 1MB)

    "`n== CPU"
    Get-CimInstance Win32_Processor | ForEach-Object {
        "{0} | {1} nhan / {2} luong | {3} MHz max" -f $_.Name.Trim(), $_.NumberOfCores, $_.NumberOfLogicalProcessors, $_.MaxClockSpeed
    }

    "`n== GPU"
    Get-CimInstance Win32_VideoController | ForEach-Object {
        "{0} | driver {1} ({2})" -f $_.Name, $_.DriverVersion, $_.DriverDate
    }

    "`n== NPU / accelerator"
    Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue |
        Where-Object { $_.FriendlyName -match 'AI Boost|NPU' } |
        ForEach-Object {
            $ver = (Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName 'DEVPKEY_Device_DriverVersion' -ErrorAction SilentlyContinue).Data
            "{0} | {1} | driver {2}" -f $_.FriendlyName, $_.Status, $ver
        }

    "`n== Pagefile"
    $managed = (Get-CimInstance Win32_ComputerSystem).AutomaticManagedPagefile
    "Windows tu quan ly: $managed"
    Get-CimInstance Win32_PageFileSetting | ForEach-Object { "Cau hinh: {0} | min {1} MB | max {2} MB" -f $_.Name, $_.InitialSize, $_.MaximumSize }
    Get-CimInstance Win32_PageFileUsage | ForEach-Object { "Dang dung: {0} | cap {1} MB | dung {2} MB | dinh {3} MB" -f $_.Name, $_.AllocatedBaseSize, $_.CurrentUsage, $_.PeakUsage }

    "`n== O dia"
    Get-PSDrive -PSProvider FileSystem | Where-Object Used | ForEach-Object {
        "{0}: trong {1:N0} GB / tong {2:N0} GB" -f $_.Name, ($_.Free / 1GB), (($_.Used + $_.Free) / 1GB)
    }
    "E:\Mori Data ton tai: $(Test-Path 'E:\Mori Data')"

    "`n== Thiet bi audio (endpoint)"
    Get-PnpDevice -Class AudioEndpoint -PresentOnly -ErrorAction SilentlyContinue |
        ForEach-Object { "{0} | {1}" -f $_.FriendlyName, $_.Status }

    "`n== Nguon dien"
    $bat = Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue
    if ($bat) { "Pin: {0}% | trang thai {1} (2 = dang cam sac)" -f $bat.EstimatedChargeRemaining, $bat.BatteryStatus }
    "Power plan: $((powercfg /getactivescheme) -replace '^.*\((.*)\).*$', '$1')"

    "`n== Toolchain"
    foreach ($c in 'git', 'node', 'cargo', 'rustc', 'python', 'uv', 'cmake') {
        $cmd = Get-Command $c -ErrorAction SilentlyContinue
        if ($cmd) { "{0}: {1}" -f $c, $cmd.Source } else { "{0}: THIEU" -f $c }
    }
} | Tee-Object -FilePath $file

"`nDa luu: $file"
