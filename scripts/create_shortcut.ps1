param(
    [Parameter(Mandatory = $true)][string]$TargetDirectory,
    [Parameter(Mandatory = $true)][string]$ShortcutName,
    [Parameter(Mandatory = $true)][string]$ExecutableName,
    [Parameter(Mandatory = $true)][string]$Description
)

$ErrorActionPreference = "Stop"
$target = Join-Path $TargetDirectory $ExecutableName
$shortcutPath = Join-Path $TargetDirectory "$ShortcutName.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $target
$shortcut.WorkingDirectory = $TargetDirectory
$shortcut.IconLocation = "$ExecutableName,0"
$shortcut.Description = $Description
$shortcut.RelativePath(".")
$shortcut.Save()
