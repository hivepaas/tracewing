# Installs tracewing with its installer, quietly, for the user alone and for
# everyone, checks where it went and what it wrote, installs it again with no
# word of whom for (where it is, as it was), and uninstalls it, checking that
# it is gone. release.yml runs it on the Windows runner, whose user is an
# administrator: the install for everyone needs no prompt there.
#
#   pwsh scripts/check_windows_installer.ps1 dist/tracewing_0.1.0_windows_x86_64-setup.exe

param([Parameter(Mandatory)][string]$Setup)

$ErrorActionPreference = 'Stop'
$Setup = (Resolve-Path $Setup).Path
$key = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\tracewing'

function Fail([string]$why) {
    Write-Host "::error::$why"
    exit 1
}

function Run([string]$exe, [string[]]$arguments) {
    $p = Start-Process -FilePath $exe -ArgumentList $arguments -Wait -PassThru
    if ($p.ExitCode -ne 0) { Fail "$exe $($arguments -join ' ') ended with $($p.ExitCode)" }
}

# The uninstall key's folder, in the 64-bit view, where the installer writes it.
function Installed([string]$hive) {
    $which = if ($hive -eq 'machine') { [Microsoft.Win32.RegistryHive]::LocalMachine } else { [Microsoft.Win32.RegistryHive]::CurrentUser }
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($which, [Microsoft.Win32.RegistryView]::Registry64)
    $k = $base.OpenSubKey($key)
    if ($null -eq $k) { return $null }
    try { return $k.GetValue('InstallLocation') } finally { $k.Close() }
}

function PathOf([string]$hive) {
    if ($hive -eq 'machine') {
        return [Environment]::GetEnvironmentVariable('Path', 'Machine')
    }
    return [Environment]::GetEnvironmentVariable('Path', 'User')
}

function Holds([string]$path, [string]$dir) {
    return @($path -split ';' | Where-Object { $_.TrimEnd('\') -ieq $dir.TrimEnd('\') }).Count -gt 0
}

$modes = @(
    @{ Flag = '/CURRENTUSER'; Hive = 'user'; Other = 'machine'; Dir = Join-Path $env:LOCALAPPDATA 'Programs\tracewing' },
    @{ Flag = '/ALLUSERS'; Hive = 'machine'; Other = 'user'; Dir = Join-Path $env:ProgramFiles 'tracewing' }
)

foreach ($m in $modes) {
    Write-Host "== $($m.Flag)"
    Run $Setup @('/S', $m.Flag)
    foreach ($file in 'tracewing.exe', 'uninstall.exe', 'LICENSE.txt', 'THIRD-PARTY-LICENSES.txt') {
        if (-not (Test-Path (Join-Path $m.Dir $file))) { Fail "$($m.Flag): no $file in $($m.Dir)" }
    }
    $at = Installed $m.Hive
    if ($at -ne $m.Dir) { Fail "$($m.Flag): the $($m.Hive)'s uninstall key says '$at', not '$($m.Dir)'" }
    if ($null -ne (Installed $m.Other)) { Fail "$($m.Flag): the $($m.Other)'s uninstall key was written too" }
    if (-not (Holds (PathOf $m.Hive) $m.Dir)) { Fail "$($m.Flag): $($m.Dir) is not in the $($m.Hive)'s PATH" }
    $said = & (Join-Path $m.Dir 'tracewing.exe') --version
    if ($LASTEXITCODE -ne 0) { Fail "$($m.Flag): tracewing --version ended with $LASTEXITCODE" }
    Write-Host "tracewing: $said"

    # Again, saying nothing of whom for: where it is, as it was.
    Run $Setup @('/S')
    if ((Installed $m.Hive) -ne $m.Dir) { Fail "$($m.Flag): installed again, it moved" }
    if ($null -ne (Installed $m.Other)) { Fail "$($m.Flag): installed again, it went for the other too" }

    # The uninstaller copies itself aside and ends at once: what it removes
    # is waited for.
    Run (Join-Path $m.Dir 'uninstall.exe') @('/S')
    $deadline = (Get-Date).AddSeconds(60)
    while ((Test-Path (Join-Path $m.Dir 'tracewing.exe')) -or ($null -ne (Installed $m.Hive))) {
        if ((Get-Date) -gt $deadline) { Fail "$($m.Flag): still there a minute after uninstalling" }
        Start-Sleep -Seconds 1
    }
    if (Holds (PathOf $m.Hive) $m.Dir) { Fail "$($m.Flag): $($m.Dir) is still in the $($m.Hive)'s PATH" }
    Write-Host "$($m.Flag): installed, again, and uninstalled"
}
