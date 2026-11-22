# S10 on Windows, the window itself (docs/design/e2e.md): it opens and draws,
# takes a key, puts a request on the clipboard, opens the save dialog, and
# closes, leaving the system proxy as it found it.
#
# Everything here is done as a hand does it -- keys pressed with keybd_event,
# the clipboard read as another program reads it, the dialog found among the
# desktop's windows -- because the window's own tests run on Linux, headless,
# and this is the one place its Windows half is seen at all.
#
# **It must run in an interactive desktop session**, as check_windows_titlebar.ps1
# must: keys go to the window in the foreground, and an ssh logon has no
# input desktop. Through e2e_windows.ps1 in a task with /IT, or at the desktop:
#
#   powershell -NoProfile -File scripts/check_windows_window.ps1 target/release/tracewing.exe
#
# It opens the window on the window's own data folder (%LOCALAPPDATA%\tracewing),
# which it sets aside first and puts back after: what was there is somebody's.
#
# Exits 0 when the window did all of it, 1 when it did not, 2 when it could not
# be run here -- which is not a pass. A verdict always carries a marker:
# `::error::` for a failure, "the window does what a hand asks of it" for a
# pass. An exit of 1 with neither is this script broken, not the app.

param([Parameter(Mandatory)][string]$Exe)

$ErrorActionPreference = 'Stop'
$Exe = (Resolve-Path $Exe).Path

function Fail([string]$why) {
    Write-Host "::error::$why"
    exit 1
}

function Cannot([string]$why) {
    Write-Host "::warning::cannot check the window here: $why"
    Write-Host 'This is not a pass. The window is unchecked.'
    exit 2
}

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public class Hand {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }

  // The visible top-level windows of a process whose class is `cls`.
  public static List<IntPtr> Of(uint pid, string cls) {
    var found = new List<IntPtr>();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) {
        var s = new StringBuilder(256); GetClassName(h, s, 256);
        if (cls == null || s.ToString() == cls) found.Add(h);
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
'@

# In the window's own pixels, so that the picture taken of it is the whole
# window: unaware, PowerShell is handed its size scaled down above 96 dpi.
if ([Hand]::SetThreadDpiAwarenessContext([IntPtr]-4) -eq [IntPtr]::Zero) {
    Cannot 'this thread cannot be made DPI-aware, so the window would be measured scaled'
}

# A key, or a chord, pressed and let go: modifiers down, the key, all up.
function Press([byte[]]$keys) {
    foreach ($k in $keys) { [Hand]::keybd_event($k, 0, 0, [IntPtr]::Zero) }
    [array]::Reverse($keys)
    foreach ($k in $keys) { [Hand]::keybd_event($k, 0, 2, [IntPtr]::Zero) }
    Start-Sleep -Milliseconds 400
}
$CTRL = 0x11; $SHIFT = 0x10

function WaitFor([string]$what, [int]$seconds, [scriptblock]$done) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while (-not (& $done)) {
        if ((Get-Date) -gt $deadline) { Fail "waited $seconds s for $what" }
        Start-Sleep -Milliseconds 250
    }
}

$settings = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'
function ProxySetting {
    $p = Get-ItemProperty $settings
    "$($p.ProxyEnable)|$($p.ProxyServer)|$($p.ProxyOverride)|$($p.AutoConfigURL)"
}

$data = Join-Path $env:LOCALAPPDATA 'tracewing'
$aside = "$data.s10-aside-$PID"
$origin = $null
$app = $null
if (Test-Path $data) { Move-Item $data $aside }
try {
    $proxyBefore = ProxySetting
    Write-Host "exe: $Exe"
    $app = Start-Process -FilePath $Exe -PassThru
    $h = [IntPtr]::Zero
    for ($i = 0; $i -lt 60; $i++) {
        Start-Sleep -Milliseconds 500
        $app.Refresh()
        if ($app.HasExited) { Cannot "the program ended with $($app.ExitCode) instead of opening a window" }
        if ($app.MainWindowHandle -ne [IntPtr]::Zero) { $h = $app.MainWindowHandle; break }
    }
    if ($h -eq [IntPtr]::Zero) { Cannot 'no window after 30 seconds' }
    [void][Hand]::SetForegroundWindow($h)
    Start-Sleep -Milliseconds 2500
    # Windows lets a process take the front only just after input of its own,
    # and this one is a hidden task: after a fresh sign-in its first ask was
    # turned down (seen in the VM at 150%). An Alt pressed and let go is that
    # input, and nothing in the window answers a lone Alt but its menu bar,
    # which tracewing does not have.
    for ($try = 0; $try -lt 3 -and [Hand]::GetForegroundWindow() -ne $h; $try++) {
        [Hand]::keybd_event(0x12, 0, 0, [IntPtr]::Zero)
        [Hand]::keybd_event(0x12, 0, 2, [IntPtr]::Zero)
        [void][Hand]::SetForegroundWindow($h)
        Start-Sleep -Milliseconds 1000
    }
    if ([Hand]::GetForegroundWindow() -ne $h) {
        Cannot 'the window cannot be brought to the front, so keys would go elsewhere (over ssh, use a task with /IT)'
    }

    # It draws: the window, as Windows composes it, is more than one colour.
    $r = New-Object Hand+RECT
    [void][Hand]::GetWindowRect($h, [ref]$r)
    $w = $r.Right - $r.Left; $hgt = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $hgt
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    [void][Hand]::PrintWindow($h, $dc, 2)
    $g.ReleaseHdc($dc); $g.Dispose()
    $colours = @{}
    for ($y = 0; $y -lt $hgt; $y += 7) { for ($x = 0; $x -lt $w; $x += 7) { $colours[$bmp.GetPixel($x, $y).ToArgb()] = 1 } }
    $bmp.Dispose()
    Write-Host "drawn: ${w}x$hgt, $($colours.Count) colours"
    if ($colours.Count -lt 8) { Fail "the window is blank: $($colours.Count) colours" }

    # It takes a key: Ctrl+2 for the traffic, Ctrl+B for the sidebar, which
    # the window writes down at once.
    Press @($CTRL, 0x32)
    Press @($CTRL, 0x42)
    $layout = Join-Path $data 'layout.toml'
    WaitFor 'Ctrl+B to hide the sidebar, written down' 10 {
        (Test-Path $layout) -and ((Get-Content -Raw $layout) -match 'sidebar_shown\s*=\s*false')
    }
    Write-Host 'Ctrl+B taken: the sidebar is hidden, and kept'
    Press @($CTRL, 0x42)

    # A request through the window's own proxy, to a server of this script's.
    $port = ([regex]::Match((& $Exe env --shell powershell | Out-String), '127\.0\.0\.1:(\d+)')).Groups[1].Value
    if (-not $port) { Fail '`tracewing env` named no port for the running window' }
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start(); $originPort = $listener.LocalEndpoint.Port; $listener.Stop()
    $origin = Start-Job -ArgumentList $originPort -ScriptBlock {
        param($p)
        $l = [System.Net.HttpListener]::new()
        $l.Prefixes.Add("http://127.0.0.1:$p/")
        $l.Start()
        while ($true) {
            $c = $l.GetContext()
            $b = [Text.Encoding]::UTF8.GetBytes('s10-answered')
            $c.Response.OutputStream.Write($b, 0, $b.Length)
            $c.Response.Close()
        }
    }
    Start-Sleep -Seconds 2
    $url = "http://127.0.0.1:$originPort/s10-window"
    $said = & curl.exe -s --max-time 10 --proxy "http://127.0.0.1:$port" $url
    if ($said -ne 's10-answered') { Fail "the request through the window's proxy came back as '$said'" }

    # It puts the request on the clipboard: the first row chosen with Home,
    # then Copy as cURL, read back as any other program reads it.
    Set-Clipboard -Value 'before'
    [void][Hand]::SetForegroundWindow($h)
    Press @(0x24)
    Press @($CTRL, $SHIFT, 0x43)
    WaitFor 'Copy as cURL on the clipboard' 10 { (Get-Clipboard -Raw) -match [regex]::Escape($url) }
    Write-Host "clipboard: $((Get-Clipboard -Raw).Split("`n")[0])"

    # It opens the save dialog, Windows' own, for the body.
    $dialogsBefore = @([Hand]::Of([uint32]$app.Id, '#32770')).Count
    Press @($CTRL, 0x53)
    WaitFor 'the save dialog' 15 { @([Hand]::Of([uint32]$app.Id, '#32770')).Count -gt $dialogsBefore }
    foreach ($d in [Hand]::Of([uint32]$app.Id, '#32770')) { [void][Hand]::PostMessageW($d, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) }
    WaitFor 'the save dialog to close' 10 { @([Hand]::Of([uint32]$app.Id, '#32770')).Count -eq $dialogsBefore }
    Write-Host 'the save dialog opened and closed'

    # It closes, and the system proxy is as it was.
    [void][Hand]::PostMessageW($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    WaitFor 'the window to close' 20 { $app.Refresh(); $app.HasExited }
    $proxyAfter = ProxySetting
    if ($proxyAfter -ne $proxyBefore) { Fail "the system proxy is not as it was: '$proxyBefore' then '$proxyAfter'" }
    Write-Host 'the window does what a hand asks of it'
    exit 0
} finally {
    if ($origin) { Stop-Job $origin; Remove-Job $origin -Force }
    if ($app -and -not $app.HasExited) { Stop-Process -Id $app.Id -Force }
    Start-Sleep -Milliseconds 500
    if (Test-Path $data) { Remove-Item -Recurse -Force $data }
    if (Test-Path $aside) { Move-Item $aside $data }
}
