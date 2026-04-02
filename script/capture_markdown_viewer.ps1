param(
    [Parameter(Mandatory = $true)]
    [string]$MarkdownPath,

    [Parameter(Mandatory = $true)]
    [string]$ScreenshotPath,

    [int]$DelayMs = 1500
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class NativeMethods {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);
}
"@

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$exePath = Join-Path $repoRoot "target\\debug\\markdown_viewer.exe"
$markdownPath = (Resolve-Path $MarkdownPath).Path
$resolvedScreenshotPath = if ([System.IO.Path]::IsPathRooted($ScreenshotPath)) {
    $ScreenshotPath
} else {
    Join-Path $repoRoot $ScreenshotPath
}
$resolvedScreenshotPath = [System.IO.Path]::GetFullPath($resolvedScreenshotPath)
$windowTitle = "{0} - Markdown Viewer" -f [System.IO.Path]::GetFileName($markdownPath)

if (-not (Test-Path $exePath)) {
    throw "viewer executable not found at $exePath"
}

$process = Start-Process -FilePath $exePath -ArgumentList @($markdownPath) -PassThru

try {
    $deadline = (Get-Date).AddSeconds(15)
    $windowProcess = $null

    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
        $windowProcess = Get-Process -Id $process.Id -ErrorAction SilentlyContinue |
            Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -eq $windowTitle }
        if ($windowProcess) {
            break
        }
    }

    if (-not $windowProcess) {
        throw "could not find viewer window '$windowTitle'"
    }

    [NativeMethods]::ShowWindowAsync($windowProcess.MainWindowHandle, 5) | Out-Null
    [NativeMethods]::SetForegroundWindow($windowProcess.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds $DelayMs

    $rect = New-Object NativeMethods+RECT
    if (-not [NativeMethods]::GetWindowRect($windowProcess.MainWindowHandle, [ref]$rect)) {
        throw "failed to read viewer window bounds"
    }

    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) {
        throw "viewer window bounds were invalid: ${width}x${height}"
    }

    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
        }
        finally {
            $graphics.Dispose()
        }

        $screenshotDirectory = Split-Path -Parent $resolvedScreenshotPath
        if ($screenshotDirectory -and -not (Test-Path $screenshotDirectory)) {
            New-Item -ItemType Directory -Path $screenshotDirectory -Force | Out-Null
        }

        $bitmap.Save($resolvedScreenshotPath, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $bitmap.Dispose()
    }

    Write-Output $resolvedScreenshotPath
}
finally {
    if ($process -and -not $process.HasExited) {
        $process.CloseMainWindow() | Out-Null
        Start-Sleep -Milliseconds 500
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
        }
    }
}
