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

---@class lunest
local M = {}

local function called_pos()
  local i = debug.getinfo(3, 'Sl')
  return {
    path = i.source:sub(2),
    line = i.currentline,
  }
end

---@param name string
---@param func function
function M.test(name, func)
  lib.test(called_pos(), name, func)
end

---@param name string
---@param func function
function M.group(name, func)
  lib.group(called_pos(), name, func)
end

---@class lunest.assert
---@overload fun(v: any, message?: string)
M.assert = setmetatable({}, {
  __call = function(_, v, message)
    lib.assert(called_pos(), v, message)
  end,
})

---@param a any
---@param b any
---@param message string
function M.assert.eq(a, b, message)
  lib.assert_eq(called_pos(), a, b, message)
end

---@param a any
---@param b any
---@param message string
function M.assert.ne(a, b, message)
  lib.assert_ne(called_pos(), a, b, message)
end

package.loaded[MODNAME] = M

lib.main()

return M
