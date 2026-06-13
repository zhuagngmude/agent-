param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

$utf8Strict = [System.Text.UTF8Encoding]::new($false, $true)
$textExtensions = @(
  ".css",
  ".html",
  ".js",
  ".json",
  ".md",
  ".ps1",
  ".txt",
  ".yaml",
  ".yml"
)
$skipParts = @(
  ".git",
  "node_modules",
  "dist",
  "build",
  "coverage",
  "data\local",
  "logs",
  "design\image2",
  "_internal"
)
function ConvertFrom-CodePoints {
  param([int[]]$CodePoints)
  return -join ($CodePoints | ForEach-Object { [char]$_ })
}

$mojibakeMarkers = @(
  (ConvertFrom-CodePoints @(0x6924, 0x572d, 0x6d30)),
  (ConvertFrom-CodePoints @(0x93b4)),
  (ConvertFrom-CodePoints @(0x93c6, 0x509b, 0x68e4)),
  (ConvertFrom-CodePoints @(0x6434, 0x65c2, 0x6564)),
  (ConvertFrom-CodePoints @(0x9365, 0x70b4, 0x7cb4)),
  (ConvertFrom-CodePoints @(0x942d, 0x30e8, 0x7611)),
  (ConvertFrom-CodePoints @(0x95c3, 0x8235)),
  (ConvertFrom-CodePoints @(0x951b, 0x3f)),
  (ConvertFrom-CodePoints @(0x9286, 0x3f)),
  (ConvertFrom-CodePoints @(0x3f, 0x2f))
)

$rootPath = (Resolve-Path $Root).Path
$rootUri = [System.Uri]::new(($rootPath.TrimEnd("\") + "\"))
$badUtf8 = New-Object System.Collections.Generic.List[string]
$badText = New-Object System.Collections.Generic.List[string]

Get-ChildItem -LiteralPath $rootPath -Recurse -File | ForEach-Object {
  $fileUri = [System.Uri]::new($_.FullName)
  $relative = [System.Uri]::UnescapeDataString($rootUri.MakeRelativeUri($fileUri).ToString()).Replace("/", "\")
  foreach ($part in $skipParts) {
    if (
      $relative.Equals($part, [System.StringComparison]::OrdinalIgnoreCase) -or
      $relative.StartsWith("${part}\", [System.StringComparison]::OrdinalIgnoreCase) -or
      ($relative.IndexOf("\${part}\", [System.StringComparison]::OrdinalIgnoreCase) -ge 0)
    ) {
      return
    }
  }
  if ($relative -eq "data\mock\runtime-state.json") {
    return
  }
  if ($relative -eq "scripts\check-encoding.ps1") {
    return
  }
  if ($textExtensions -notcontains $_.Extension.ToLowerInvariant()) {
    return
  }

  $bytes = [System.IO.File]::ReadAllBytes($_.FullName)
  try {
    $text = $utf8Strict.GetString($bytes)
  } catch {
    $badUtf8.Add($relative)
    return
  }

  foreach ($marker in $mojibakeMarkers) {
    if ($text.Contains($marker)) {
      $badText.Add("${relative}: ${marker}")
      return
    }
  }
}

if ($badUtf8.Count -gt 0 -or $badText.Count -gt 0) {
  if ($badUtf8.Count -gt 0) {
    Write-Host "Invalid UTF-8 files:"
    $badUtf8 | ForEach-Object { Write-Host " - $_" }
  }
  if ($badText.Count -gt 0) {
    Write-Host "Possible mojibake text:"
    $badText | ForEach-Object { Write-Host " - $_" }
  }
  exit 1
}

Write-Host "Encoding check passed: UTF-8 text files, no known mojibake markers."
