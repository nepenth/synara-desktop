# Waits for Synara's window to show the sign-in heading, read through Windows
# UI Automation (WebView2 publishes the page's accessibility tree there), then
# saves a desktop screenshot and the WebView2 command lines to -Out.
# Exits 0 when the heading appeared, 1 when it did not within -TimeoutSeconds.
param(
  [Parameter(Mandatory = $true)][string]$Out,
  [string]$Expected = 'Welcome back',
  [int]$TimeoutSeconds = 90
)

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

$automation = [System.Windows.Automation.AutomationElement]
$scope = [System.Windows.Automation.TreeScope]
$nameIs = {
  param($name)
  New-Object System.Windows.Automation.PropertyCondition($automation::NameProperty, $name)
}

$found = $false
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
while (-not $found -and (Get-Date) -lt $deadline) {
  Start-Sleep -Seconds 1
  $window = $automation::RootElement.FindFirst($scope::Children, (& $nameIs 'Synara'))
  if ($null -eq $window) { continue }
  $heading = $window.FindFirst($scope::Descendants, (& $nameIs $Expected))
  if ($null -ne $heading) { $found = $true }
}

New-Item -ItemType Directory -Force -Path $Out | Out-Null
$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
$bitmap.Save((Join-Path $Out 'synara-windows.png'), [System.Drawing.Imaging.ImageFormat]::Png)
Get-CimInstance Win32_Process |
  Where-Object { $_.Name -match 'Synara|msedgewebview2' } |
  ForEach-Object { "$($_.ProcessId) $($_.Name) $($_.CommandLine)" } |
  Out-File -Encoding utf8 (Join-Path $Out 'processes.txt')

if ($found) {
  Write-Output "[windows-ui-smoke] the Synara window shows `"$Expected`"."
  exit 0
}
Write-Output "[windows-ui-smoke] the Synara window never showed `"$Expected`" within $TimeoutSeconds s."
exit 1
