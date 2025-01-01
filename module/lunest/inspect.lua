local M = {}
package.loaded[...] = M

local bridge = require("lunest.bridge")

local Value = require("lunest.inspect.Value")

---@param any any
---@param max_width integer?
---@return string
function M.inspect(any, max_width)
    return Value.new(any):fmt_wrap():display(max_width or bridge.get_term_width())
end

return M
