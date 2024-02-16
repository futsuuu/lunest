local MODNAME = 'lunest'

if package.loaded[MODNAME] then
  return
end

---@return string
local function lua_id()
  local str = ''
  for c in _VERSION:gmatch('%d+') do
    str = str .. c
  end
  return str
end

local function load_dll()
  local dll = (debug.getinfo(1, 'S').source:match('@(.+[/\\])') or '')
    .. 'lunest_lib.'
    .. lua_id()
    .. (package.path:sub(1, 1) == '/' and '.so' or '.dll')
  local loader = assert(package.loadlib(dll, 'luaopen_lunest_lib'))
  return loader()
end

local lib = load_dll()

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

package.loaded[MODNAME] = M

lib.main()
