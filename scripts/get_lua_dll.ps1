function Get-LuaDLL {
  Param (
    [ValidateSet("51", "52", "53", "54")]$version
  )
  $semver, $dll = switch ($version) {
    "51" { "5.1.5", "lua5.1.dll" }
    "52" { "5.2.4", "lua52.dll" }
    "53" { "5.3.6", "lua53.dll" }
    "54" { "5.4.2", "lua54.dll" }
  }
  $uri = "https://downloads.sourceforge.net/project/luabinaries/${semver}/Windows%20Libraries/Dynamic/lua-${semver}_Win64_dll17_lib.zip"
  Invoke-WebRequest -Uri $uri -Method 'GET' -Outfile 'lualib.zip' -UserAgent 'PowerShell'
  Expand-Archive -Force -Path 'lualib.zip' -DestinationPath 'lualib'

  Move-Item -Force "lualib/${dll}" "lua${version}.dll"
  Remove-Item -Recurse -Force 'lualib.zip', 'lualib'
}

Get-LuaDLL "51"
Get-LuaDLL "52"
Get-LuaDLL "53"
Get-LuaDLL "54"
