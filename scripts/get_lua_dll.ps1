Param(
  [ValidateSet("5.1", "5.2", "5.3", "5.4")]$version
)

$semver, $dll = switch ($version) {
  "5.1" { "5.1.5", "lua5.1.dll" }
  "5.2" { "5.2.4", "lua52.dll" }
  "5.3" { "5.3.6", "lua53.dll" }
  "5.4" { "5.4.2", "lua54.dll" }
}
$uri = "https://downloads.sourceforge.net/project/luabinaries/${semver}/Windows%20Libraries/Dynamic/lua-${semver}_Win64_dll17_lib.zip"
Invoke-WebRequest -Uri $uri -Method 'GET' -Outfile 'lualib.zip' -UserAgent 'PowerShell'
Expand-Archive -Force -Path 'lualib.zip' -DestinationPath 'lualib'

Move-Item -Force "lualib/${dll}" "lua$($version -replace '\.', '').dll"

Remove-Item -Recurse -Force 'lualib.zip', 'lualib'
