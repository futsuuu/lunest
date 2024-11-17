local test = require("lunest.wrapper")

local Fmt = require("lunest.inspect.Fmt")

local ROOT_VALUE = "(root)"
local INDENT = "  "

---@generic T: table
---@param t T
---@param dst table?
---@param opts { metatable: boolean? }?
---@return T
local function copy(t, dst, opts)
    opts = opts or {}
    local r = dst or {}
    for key, value in pairs(t) do
        r[key] = value
    end
    if opts.metatable ~= false then
        setmetatable(r, getmetatable(t))
    end
    return r
end

local sort
do
    local TYPE_ORDERS = {
        ["number"] = 1,
        ["boolean"] = 2,
        ["string"] = 3,
        ["table"] = 4,
        ["function"] = 5,
        ["thread"] = 6,
        ["userdata"] = 7,
    }
    ---@generic T
    ---@param list T[]
    ---@param value nil | fun(e: T): any
    function sort(list, value)
        table.sort(list, function(a, b)
            if value then
                a, b = value(a), value(b)
            end
            local ta, tb = type(a), type(b)
            if ta ~= tb then
                return TYPE_ORDERS[ta] < TYPE_ORDERS[tb]
            elseif ta == "number" or ta == "string" then
                return a < b
            else
                return false
            end
        end)
    end
end

local is_keyword
do
    local KEYWORDS = {
        ["and"] = true,
        ["break"] = true,
        ["do"] = true,
        ["else"] = true,
        ["elseif"] = true,
        ["end"] = true,
        ["false"] = true,
        ["for"] = true,
        ["function"] = true,
        ["goto"] = _VERSION ~= "Lua 5.1",
        ["if"] = true,
        ["in"] = true,
        ["local"] = true,
        ["nil"] = true,
        ["not"] = true,
        ["or"] = true,
        ["repeat"] = true,
        ["return"] = true,
        ["then"] = true,
        ["true"] = true,
        ["until"] = true,
        ["while"] = true,
    }
    ---@param s string
    ---@return boolean
    function is_keyword(s)
        return KEYWORDS[s] == true
    end
end

---@class lunest.inspect.Context
---@field current_path any[]
---@field path_by_ref table<any, any[]>
local Context = {}
---@private
Context.__index = Context

---@return self
function Context.new()
    ---@type lunest.inspect.Context
    local self = {
        current_path = {},
        path_by_ref = {},
    }
    return setmetatable(self, Context)
end

---@return self
function Context:snapshot()
    ---@type lunest.inspect.Context
    return {
        current_path = copy(self.current_path),
        path_by_ref = copy(self.path_by_ref),
    }
end

---@param snapshot self
function Context:reset(snapshot)
    copy(snapshot, self, { metatable = false })
end

---@generic F: function
---@param snapshot self
---@param f F
---@return F
function Context:with_reset(snapshot, f)
    return function(...)
        self:reset(snapshot)
        return f(...)
    end
end

---@param obj any
---@return boolean
function Context:is_new_reference(obj)
    if self.path_by_ref[obj] then
        return false
    end
    self.path_by_ref[obj] = copy(self.current_path)
    return true
end

local inspect

---@param object any
---@param cx lunest.inspect.Context?
---@return lunest.inspect.Fmt
---@return boolean raw
local function table_key(object, cx)
    cx = cx or Context.new()
    if type(object) == "string" and not is_keyword(object) and object:match("^[_%a][_%w]*$") then
        return Fmt.str(object), true
    else
        return Fmt.new(function()
            return { "[", inspect(object, cx), "]" }
        end),
            false
    end
end

test.test("table_key", function()
    assert("[1]" == table_key(1):tostring())
    assert("hello" == table_key("hello"):tostring())
    assert('["end"]' == table_key("end"):tostring())
end)

---@param path any[]
---@param cx lunest.inspect.Context?
---@return lunest.inspect.Fmt
local function display_path(path, cx)
    cx = cx or Context.new()

    ---@param folded boolean
    ---@return (string | lunest.inspect.Fmt)[]
    local function fmt(folded)
        local r = { ROOT_VALUE }
        for _, obj in ipairs(path) do
            if not folded then
                table.insert(r, "\n" .. INDENT)
            end
            local key, raw = table_key(obj, cx)
            if raw then
                table.insert(r, ".")
            end
            table.insert(r, key)
        end
        return r
    end

    return Fmt.new(
        function()
            return fmt(true)
        end,
        cx:with_reset(cx:snapshot(), function()
            return fmt(false)
        end)
    )
end

test.test("display_path", function()
    assert(
        '(root)[0].hello.world["!"]._'
            == display_path({ 0, "hello", "world", "!", "_" }):tostring(28)
    )
    assert([[
(root)
  [0]
  .hello
  .world
  ["!"]
  ._]] == display_path({ 0, "hello", "world", "!", "_" }):tostring(27))
end)

---@param object any
---@param cx lunest.inspect.Context?
---@return lunest.inspect.Fmt
function inspect(object, cx)
    cx = cx or Context.new()

    local ty = type(object)

    if ty == "nil" or ty == "boolean" or ty == "number" then
        return Fmt.str(tostring(object))
    elseif ty == "string" then
        return Fmt.str((("%q"):format(object):gsub("\\\n", "\\n")))
    end

    if not cx:is_new_reference(object) then
        return display_path(cx.path_by_ref[object], cx)
    end

    if ty == "table" then
        if not next(object) then
            return Fmt.str("{}")
        end

        ---@type { key: any, value: any }[]
        local list = {}
        for key, value in pairs(object) do
            table.insert(list, { key = key, value = value })
        end
        sort(list, function(e)
            return e.key
        end)

        ---@param folded boolean
        ---@return (string | lunest.inspect.Fmt)[]
        local function fmt(folded)
            ---@type (string | lunest.inspect.Fmt)[]
            local r = { folded and "{ " or "{\n" }

            local prev_key = nil
            for _, e in ipairs(list) do
                if not folded then
                    table.insert(r, INDENT)
                end
                if
                    (type(prev_key) == "number" or prev_key == nil)
                    and e.key == (prev_key or 0) + 1
                then
                    prev_key = e.key
                else
                    prev_key = nil
                    table.insert(r, (table_key(e.key, cx)))
                    table.insert(r, " = ")
                end
                table.insert(cx.current_path, e.key)
                table.insert(r, inspect(e.value, cx))
                table.remove(cx.current_path)
                table.insert(r, folded and ", " or ",\n")
            end
            if folded then
                table.remove(r)
            end

            table.insert(r, folded and " }" or "}")

            return r
        end

        return Fmt.new(
            function()
                return fmt(true)
            end,
            cx:with_reset(cx:snapshot(), function()
                return fmt(false)
            end)
        )
    end

    if ty == "function" then
        local info = debug.getinfo(object, "S")
        if info.what == "Lua" and info.short_src ~= "" then
            local s = info.short_src
            if 0 < (info.linedefined or 0) then
                s = s .. ":" .. info.linedefined
            end
            return Fmt.str(("(func %q)"):format(s))
        else
            return Fmt.str(("(func %q)"):format(info.what))
        end
    end

    return Fmt.str(("(%s)"):format(ty))
end

test.test("empty table", function()
    assert("{}" == inspect({}):tostring())
end)

test.test("list", function()
    assert(
        '{ [0] = "x", "a", "b", [4] = "d", [5] = "e" }'
            == inspect({ [0] = "x", "a", "b", nil, "d", "e" }):tostring()
    )
end)

test.test("recursive", function()
    local t = {}
    t.M = {}
    t.M.__index = t.M
    assert("{ M = { __index = (root).M } }" == inspect(t):tostring())
end)

test.test("reference", function()
    local t = { a = { b = {} } }
    t.b = t.a.b
    assert("{ a = { b = {} }, b = (root).a.b }" == inspect(t):tostring())
end)
-- local t = { a = { b = {} } }
-- t.b = t.a.b
-- print(inspect(t):tostring())

test.test("expand key", function()
    assert([[
{
  [{
    a = 1,
    b = 2,
  }] = true,
}]] == inspect({ [{ a = 1, b = 2 }] = true }):tostring(12))
end)

-- print(inspect(_G):tostring(157))

return function(obj)
    return inspect(obj):tostring(80)
end
