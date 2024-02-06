Param(
  [ValidateSet("5.1", "5.2", "5.3", "5.4")]$version
)

$semver = switch ($version) {
  "5.1" { "5.1.5" }
  "5.2" { "5.2.4" }
  "5.3" { "5.3.6" }
  "5.4" { "5.4.2" }
}
$uri = "https://downloads.sourceforge.net/project/luabinaries/${semver}/Windows%20Libraries/Dynamic/lua-${semver}_Win64_dll17_lib.zip"
Invoke-WebRequest -Uri $uri -Method 'GET' -Outfile 'lualib.zip' -UserAgent 'PowerShell'
Expand-Archive -Force -Path 'lualib.zip' -DestinationPath 'lualib'

$dll = "lua$($version -replace '\.', '').dll"
Move-Item -Force "lualib/lua${version}.dll" $dll

Remove-Item -Recurse -Force 'lualib.zip', 'lualib'
