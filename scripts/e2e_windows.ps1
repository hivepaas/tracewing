# Runs crates/e2e's tests, the ones that change the computer they run on —
# its proxy setting, its certificate stores, its Credential Manager, its
# firewall — on this Windows machine. Only on a machine meant for it: a VM or
# a CI runner. Say so with -ThisMachineMayBeChanged, which makes the marker
# file the tests look for (C:\tracewing-e2e-throwaway); without the marker
# every test returns at once.
#
#   pwsh scripts/e2e_windows.ps1 -ThisMachineMayBeChanged
#   pwsh scripts/e2e_windows.ps1 -ThisMachineMayBeChanged -SkipBuild
#
# Run it as an administrator: the machine's certificate store and the
# firewall take one.
param(
    [switch]$ThisMachineMayBeChanged,
    [switch]$SkipBuild
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
$tests = cargo test --release --locked -p tracewing-e2e --no-run --message-format=json |
    ForEach-Object { $_ | ConvertFrom-Json } |
    Where-Object { $_.reason -eq 'compiler-artifact' -and $_.executable -and ($_.target.kind -contains 'test') } |
    ForEach-Object { $_.executable }
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$env:TRACEWING_E2E = '1'
$env:TRACEWING_E2E_BIN = (Resolve-Path "$target\release\tracewing.exe").Path
$status = 0
foreach ($test in $tests) {
    Write-Output "== $(Split-Path -Leaf $test)"
    & $test --test-threads=1
    if ($LASTEXITCODE -ne 0) { $status = 1 }
}
exit $status
