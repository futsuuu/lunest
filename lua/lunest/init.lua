do
  local modname = 'lunest.lib'
  if not package.preload[modname] and not package.loaded[modname] then
    local dll = (debug.getinfo(1, 'S').source:match('@(.+[/\\])') or '')
      .. 'lunest'
      .. (package.path:sub(1, 1) == '/' and '.so' or '.dll')

    local loader = assert(package.loadlib(dll, 'luaopen_lunest'))
    package.preload[modname] = loader
  end
end

if arg[0] then
  require('lunest.lib').cli({ arg[0], (unpack or table.unpack)(arg) })
  return
end
