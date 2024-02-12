assert(arg[0])

local lib_modname = 'lunest.lib'
assert(not package.loaded[lib_modname])
do
  local dll = (debug.getinfo(1, 'S').source:match('@(.+[/\\])') or '')
    .. 'lunest_lib'
    .. (package.path:sub(1, 1) == '/' and '.so' or '.dll')

  local loader = assert(package.loadlib(dll, 'luaopen_lunest'))
  package.loaded[lib_modname] = loader()
end

local lib = require(lib_modname)

local M = {}

---Return a root element of the NodeID (same as file path) to determine that
---the function was called from the same file.
local function where_called()
  return debug.getinfo(3, 'S').source:sub(2)
end

---@param name string
---@param func function
function M.test(name, func)
  lib.test(where_called(), name, func)
end

---@param name string
---@param func function
function M.group(name, func)
  lib.group(where_called(), name, func)
end

package.loaded.lunest = M

lib.main({ arg[0], (unpack or table.unpack)(arg) })
