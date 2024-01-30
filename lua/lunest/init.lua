local lib_modname = 'lunest.lib'
assert(arg[0] and not package.loaded[lib_modname])
do
  local dll = (debug.getinfo(1, 'S').source:match('@(.+[/\\])') or '')
    .. 'lunest'
    .. (package.path:sub(1, 1) == '/' and '.so' or '.dll')

  local loader = assert(package.loadlib(dll, 'luaopen_lunest'))
  package.loaded[lib_modname] = loader()
end

local lib = require('lunest.lib')

local M = {}

---@param name string
---@param func function
function M.test(name, func)
  lib.test(name, func)
end

---@param name string
---@param func function
function M.group(name, func)
  lib.group(name, func)
end

package.loaded.lunest = M

lib.main({ arg[0], (unpack or table.unpack)(arg) })
