---@class lunest.inspect.Fmt
---@field package folded fun(): ((string | lunest.inspect.Fmt)[])
---@field package expanded? fun(): ((string | lunest.inspect.Fmt)[])
local M = {}

local test = require("lunest.wrapper")

---@private
M.__index = M

---@param folded fun(): ((string | lunest.inspect.Fmt)[])
---@param expanded nil | fun(): ((string | lunest.inspect.Fmt)[])
---@return self
function M.new(folded, expanded)
    ---@type lunest.inspect.Fmt
    local self = {
        folded = folded,
        expanded = expanded,
    }
    return setmetatable(self, M)
end

---@param folded string
---@param expanded string?
---@return self
function M.str(folded, expanded)
    return M.new(function()
        return { folded }
    end, expanded and function()
        return { expanded }
    end)
end

---@param fn fun(folded: boolean): ((string | lunest.inspect.Fmt)[])
function M.fn(fn)
    return M.new(function()
        return fn(true)
    end, function()
        return fn(false)
    end)
end

---@param str string
---@return string
local function get_lastline(str)
    return str:match("\n([^\n]*)$") or str
end

---@param str string
---@param prefix string
---@return string
local function trim_start(str, prefix)
    return str:sub(#prefix + 1)
end

---@param str string
---@return string indent
---@return string dedented
local function get_indent(str)
    local indent = str:match("^(%s*)") ---@type string
    return indent, trim_start(str, indent)
end

---@param max_width integer
---@param lastline string
---@param expand boolean?
---@return string
function M:_tostring(max_width, lastline, expand)
    expand = expand == true
    if expand then
        assert(self.expanded)
    end
    local is_expandable = not expand and self.expanded ~= nil

    local result = lastline
    local indent = get_indent(lastline)
    for _, child in ipairs(expand and self.expanded() or self.folded()) do
        local s
        if type(child) == "string" then
            s = child:gsub("\n", "\n" .. indent)
        else
            s = child:_tostring(max_width, get_lastline(result))
        end

        result = result .. s
        if is_expandable and (s:find("\n") or max_width < #result) then
            return self:_tostring(max_width, lastline, true)
        end
    end

    return trim_start(result, lastline)
end

---@param max_width integer?
---@return string
function M:tostring(max_width)
    return self:_tostring(max_width or math.huge, "")
end

test.group("tostring", function()
    test.test("expand", function()
        assert("12345" == M.str("12345", "1234\n5"):tostring(5))
        assert("1234\n5" == M.str("12345", "1234\n5"):tostring(4))
    end)

    test.test("indent", function()
        assert([[
  hello
    world]] == M.new(function()
            return { "  hello", M.str("\n  world") }
        end):tostring(1))
    end)
end)

return M
