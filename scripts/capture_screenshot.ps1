Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Start-Sleep -Seconds 3

# Find window handle for SonicForge DSP
$proc = Get-Process -Name "sonicforge-dsp" -ErrorAction SilentlyContinue | Select-Object -First 1

if ($proc -and $proc.MainWindowHandle -ne [IntPtr]::Zero) {
    Write-Output "Found SonicForge DSP window handle: $($proc.MainWindowHandle)"
    
    # Bring window to foreground
    $sig = @'
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }
'@
    $user32 = Add-Type -MemberDefinition $sig -Name "User32" -Namespace "Win32" -PassThru
    $user32::ShowWindow($proc.MainWindowHandle, 9) # SW_RESTORE
    $user32::SetForegroundWindow($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 800

    $rect = New-Object Win32.User32+RECT
    $user32::GetWindowRect($proc.MainWindowHandle, [ref]$rect)

    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top

    if ($width -gt 0 -and $height -gt 0) {
        $bitmap = New-Object System.Drawing.Bitmap($width, $height)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, [System.Drawing.Size]::new($width, $height))
        
        $outputDir = "c:\ProjectO\equalizer-win\walkthrough_assets"
        if (!(Test-Path $outputDir)) {
            New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
        }
        $outputPath = Join-Path $outputDir "sonicforge_app.png"
        $bitmap.Save($outputPath, [System.Drawing.Imaging.ImageFormat]::Png)
        $graphics.Dispose()
        $bitmap.Dispose()
        Write-Output "Window screenshot successfully saved to $outputPath"
        exit 0
    }
}

# Fallback: Capture primary screen
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($bounds.X, $bounds.Y, 0, 0, $bounds.Size)

$outputDir = "c:\ProjectO\equalizer-win\walkthrough_assets"
if (!(Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}
$outputPath = Join-Path $outputDir "sonicforge_app.png"
$bitmap.Save($outputPath, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
Write-Output "Primary screen screenshot saved to $outputPath"
