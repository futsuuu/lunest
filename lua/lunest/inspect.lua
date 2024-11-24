local test = require("lunest.wrapper")

local Fmt = require("lunest.inspect.Fmt")

local ROOT_VALUE = "(root)"
local INDENT = "  "

---@generic T: table
---@param t T
---@return T
local function copy(t)
    local r = {}
    for k, v in pairs(t) do
        r[k] = v
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

local format

---@param object any
---@return lunest.inspect.Fmt
---@return boolean raw
local function table_key(object)
    if type(object) == "string" and not is_keyword(object) and object:match("^[_%a][_%w]*$") then
        return Fmt.str(object), true
    else
        return Fmt.new(function()
            return { "[", format(object), "]" }
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
local function display_path(path)
    return Fmt.fn(function(folded)
        local r = { ROOT_VALUE }
        for _, obj in ipairs(path) do
            if not folded then
                table.insert(r, "\n" .. INDENT)
            end
            local key, raw = table_key(obj)
            if raw then
                table.insert(r, ".")
            end
            table.insert(r, key)
        end
        return r
    end)
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

local path_mt = {}

---@param obj any
---@param current_path? any[]
---@param path_by_ref? table<any, any[]>
---@return any
local function transform(obj, current_path, path_by_ref)
    current_path, path_by_ref = current_path or {}, path_by_ref or {}
    if path_by_ref[obj] then
        return path_by_ref[obj]
    end
    local ty = type(obj)
    if ty == "table" or ty == "function" or ty == "thread" or ty == "userdata" then
        path_by_ref[obj] = setmetatable(copy(current_path), path_mt)
    end
    if ty ~= "table" then
        return obj
    end
    ---@type { key: any, value: any }[]
    local list = {}
    for key, value in pairs(obj) do
        table.insert(list, { key = key, value = value })
    end
    sort(list, function(e)
        return e.key
    end)
    for _, e in ipairs(list) do
        local key = transform(e.key, {}, setmetatable({}, { __index = path_by_ref }))
        table.insert(current_path, key)
        local value = transform(e.value, current_path, path_by_ref)
        table.remove(current_path)
        e.key, e.value = key, value
    end
    return setmetatable(list, getmetatable(obj))
end

---@param object any
---@return lunest.inspect.Fmt
function format(object)
    local ty = type(object)

    if ty == "nil" or ty == "boolean" or ty == "number" then
        return Fmt.str(tostring(object))
    elseif ty == "string" then
        return Fmt.str((("%q"):format(object):gsub("\\\n", "\\n")))
    elseif ty == "function" then
        local info = debug.getinfo(object, "S")
        if info.what ~= "Lua" or info.short_src == "" then
            return Fmt.str(("(func %q)"):format(info.what))
        end
        local s = info.short_src
        if 0 < (info.linedefined or 0) then
            s = s .. ":" .. info.linedefined
        end
        return Fmt.str(("(func %q)"):format(s))
    elseif ty ~= "table" then
        return Fmt.str(("(%s)"):format(ty))
    end

    if getmetatable(object) == path_mt then
        return display_path(object)
    end
    ---@cast object { key: any, value: any }[]
    if not next(object) then
        return Fmt.str("{}")
    end

    return Fmt.fn(function(folded)
        ---@type (string | lunest.inspect.Fmt)[]
        local r = { folded and "{ " or "{\n" }

        local prev_key = nil
        for _, e in ipairs(object) do
            if not folded then
                table.insert(r, INDENT)
            end
            if (type(prev_key) == "number" or prev_key == nil) and e.key == (prev_key or 0) + 1 then
                prev_key = e.key
            else
                prev_key = nil
                table.insert(r, (table_key(e.key)))
                table.insert(r, " = ")
            end
            table.insert(r, format(e.value))
            table.insert(r, folded and ", " or ",\n")
        end
        if folded then
            table.remove(r)
        end
        table.insert(r, folded and " }" or "}")

        return r
    end)
end

---@param obj any
---@param max_width integer?
---@return string
local function inspect(obj, max_width)
    return format(transform(obj)):tostring(max_width)
end

test.test("empty table", function()
    assert("{}" == inspect({}))
end)

test.test("list", function()
    assert(
        '{ [0] = "x", "a", "b", [4] = "d", [5] = "e" }'
            == inspect({ [0] = "x", "a", "b", nil, "d", "e" })
    )
end)

test.test("recursive", function()
    local t = {}
    t.M = {}
    t.M.__index = t.M
    assert("{ M = { __index = (root).M } }" == inspect(t))
end)

test.test("reference", function()
    local t = { a = { b = {} } }
    t.b = t.a.b
    assert("{ a = { b = {} }, b = (root).a.b }" == inspect(t))
end)

test.test("expand key", function()
    assert([[
{
  [{
    a = 1,
    b = 2,
  }] = true,
}]] == inspect({ [{ a = 1, b = 2 }] = true }, 12))
end)

return function(obj)
    return inspect(obj, 80)
end
