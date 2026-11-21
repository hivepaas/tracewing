# Runs crates/e2e's tests, the ones that change the computer they run on —
# its proxy setting, its certificate stores, its Credential Manager, its
# firewall — on this Windows machine. Only on a machine meant for it: a VM or
# a CI runner. Say so with -ThisMachineMayBeChanged, which makes the marker
# file the tests look for (C:\tracewing-e2e-throwaway); without the marker
# every test returns at once.
#
#   pwsh scripts/e2e_windows.ps1 -ThisMachineMayBeChanged
#   pwsh scripts/e2e_windows.ps1 -ThisMachineMayBeChanged -SkipBuild
#   pwsh scripts/e2e_windows.ps1 -ThisMachineMayBeChanged -Packages -Browsers 'chrome edge firefox'
#
# -Packages builds the installer and the zip (scripts/package_windows.sh, in
# Git Bash, with NSIS and cargo-about) for the tests that install them.
# -Browsers names the browsers installed here for the tests that open them.
# -Wintun is the path to wintun.dll: with it, the live tests of capture with
# no proxy settings run too (S8), which put an adapter on this machine for
# as long as each runs. They need an administrator, and not the desktop.
#
# Run it as an administrator: the machine's certificate store and the
# firewall take one. And in a desktop session, as a user runs tracewing: a
# session with no desktop, as ssh gives, can keep nothing in the Credential
# Manager ("Credentials cannot be saved from this logon session") and add
# nothing to this user's root store.
#
# Putting a root in this user's store, or taking it out, makes Windows ask
# the user. -ConfirmTrustDialogs answers Yes to those two questions and no
# other, as the user would, so that the step can be seen through.
param(
    [switch]$ThisMachineMayBeChanged,
    [switch]$SkipBuild,
    [switch]$ConfirmTrustDialogs,
    [switch]$Packages,
    [string]$Browsers = '',
    [string]$Wintun = ''
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Set-Location (Join-Path $PSScriptRoot '..')

$marker = 'C:\tracewing-e2e-throwaway'
if ($ThisMachineMayBeChanged) {
    New-Item -ItemType File -Force $marker | Out-Null
} elseif (-not (Test-Path $marker)) {
    Write-Output "Not marked as a machine these tests may change: pass -ThisMachineMayBeChanged on a VM or a CI runner."
    exit 1
}

$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
if (-not $SkipBuild) {
    cargo build --release --locked -p tracewing
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
$dist = "$target\e2e-dist"
if ($Packages) {
    $bash = 'C:\Program Files\Git\bin\bash.exe'
    if (-not (Test-Path $bash)) { Write-Output "-Packages needs Git Bash at $bash"; exit 1 }
    Remove-Item -Recurse -Force $dist -ErrorAction SilentlyContinue
    $env:EXE = "$target\release\tracewing.exe"
    $env:DIST = $dist
    & $bash scripts/package_windows.sh
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $env:TRACEWING_E2E_SETUP = (Get-Item "$dist\tracewing_*-setup.exe").FullName
    $env:TRACEWING_E2E_ZIP = (Get-Item "$dist\tracewing_*.zip").FullName
    Write-Output "packages: $env:TRACEWING_E2E_SETUP $env:TRACEWING_E2E_ZIP"
}
if ($Browsers) { $env:TRACEWING_E2E_BROWSERS = $Browsers }

$tests = cargo test --release --locked -p tracewing-e2e --no-run --message-format=json |
    ForEach-Object { $_ | ConvertFrom-Json } |
    Where-Object { $_.reason -eq 'compiler-artifact' -and $_.executable -and ($_.target.kind -contains 'test') } |
    ForEach-Object { $_.executable }
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$confirmer = $null
if ($ConfirmTrustDialogs) {
    # The two dialogs are plain ones, whose buttons UI Automation sees only as
    # panes with nothing to invoke, so they are answered as Windows' own
    # controls: Yes is the button with the dialog id IDYES, pressed with
    # BM_CLICK, and only in a dialog that names tracewing's CA.
    $confirmer = Start-Job -ScriptBlock {
        Add-Type -Namespace E2E -Name Answer -MemberDefinition @'
delegate bool EnumProc(System.IntPtr h, System.IntPtr l);
[System.Runtime.InteropServices.DllImport("user32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
static extern System.IntPtr FindWindow(string c, string n);
[System.Runtime.InteropServices.DllImport("user32.dll")]
static extern bool EnumChildWindows(System.IntPtr p, EnumProc f, System.IntPtr l);
[System.Runtime.InteropServices.DllImport("user32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
static extern int GetWindowText(System.IntPtr h, System.Text.StringBuilder s, int n);
[System.Runtime.InteropServices.DllImport("user32.dll")]
static extern System.IntPtr GetDlgItem(System.IntPtr d, int id);
[System.Runtime.InteropServices.DllImport("user32.dll")]
static extern System.IntPtr SendMessage(System.IntPtr h, int m, System.IntPtr w, System.IntPtr l);
public static bool Yes(string title, string mention) {
    System.IntPtr dialog = FindWindow("#32770", title);
    if (dialog == System.IntPtr.Zero) return false;
    var said = new System.Text.StringBuilder();
    EnumChildWindows(dialog, (h, l) => {
        var text = new System.Text.StringBuilder(4096);
        GetWindowText(h, text, 4096);
        said.Append(text).Append('\n');
        return true;
    }, System.IntPtr.Zero);
    if (!said.ToString().Contains(mention)) return false;
    System.IntPtr yes = GetDlgItem(dialog, 6);
    if (yes == System.IntPtr.Zero) return false;
    SendMessage(yes, 0x00F5, System.IntPtr.Zero, System.IntPtr.Zero);
    return true;
}
'@
        while ($true) {
            foreach ($title in 'Security Warning', 'Root Certificate Store') {
                if ([E2E.Answer]::Yes($title, 'tracewing CA')) {
                    Write-Output "answered Yes to $title"
                }
            }
            Start-Sleep -Milliseconds 200
        }
    }
}

# Roots of tracewing's left by an earlier run that failed before it could
# take them out: on a machine marked for these tests, they are the tests'.
# Taking one out of this user's store asks, which the confirmer answers.
if ($ConfirmTrustDialogs) {
    foreach ($store in 'Cert:\CurrentUser\Root', 'Cert:\LocalMachine\Root') {
        Get-ChildItem $store | Where-Object { $_.Subject -like '*tracewing CA*' } | ForEach-Object {
            Write-Output "left by an earlier run, taken out: $store $($_.Thumbprint)"
            Remove-Item $_.PSPath
        }
    }
}

$env:TRACEWING_E2E = '1'
$env:TRACEWING_E2E_BIN = (Resolve-Path "$target\release\tracewing.exe").Path
$status = 0
foreach ($test in $tests) {
    Write-Output "== $(Split-Path -Leaf $test)"
    & $test --test-threads=1
    if ($LASTEXITCODE -ne 0) { $status = 1 }
}
if ($Wintun) {
    # One at a time: they share one adapter, by a fixed GUID.
    $env:WINTUN_DLL = (Resolve-Path $Wintun).Path
    foreach ($live in @(
            @('-p', 'tracewing-proxy', '--test', 'wintun_live'),
            @('-p', 'tracewing', '--test', 'local_live'))) {
        Write-Output "== $($live -join ' ')"
        cargo test --release --locked @live -- --ignored --test-threads=1
        if ($LASTEXITCODE -ne 0) { $status = 1 }
    }
}
# S11, the title bar's three caption buttons. Nothing else in this suite can
# see them: they are painted by tracewing and exist to Windows only as the
# answer to a hit test, so no test on any platform presses them. It moves the
# real pointer, so exit 2 means "no input desktop here" -- an ssh logon has
# none -- and is not a failure of tracewing's.
Write-Output '== the title bar'
& powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot\check_windows_titlebar.ps1" $env:TRACEWING_E2E_BIN
switch ($LASTEXITCODE) {
    0 { }
    2 { Write-Output 'the title bar is unchecked: this session has no input desktop' }
    default { $status = 1 }
}

# S10, the window: it opens and draws, takes a key, puts a request on the
# clipboard, opens the save dialog, and closes with the system proxy as it
# was. Keys go to the window in front, so it wants the input desktop too, and
# exit 2 is "not here", as above.
Write-Output '== the window'
& powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot\check_windows_window.ps1" $env:TRACEWING_E2E_BIN
switch ($LASTEXITCODE) {
    0 { }
    2 { Write-Output 'the window is unchecked: this session has no input desktop' }
    default { $status = 1 }
}

if ($confirmer) {
    Stop-Job $confirmer
    Receive-Job $confirmer | ForEach-Object { Write-Output "dialogs: $_" }
    Remove-Job $confirmer
}
exit $status
