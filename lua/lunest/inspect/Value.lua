local M = {}
package.loaded[...] = M

local test = require("lunest.wrapper")

local assertion = require("lunest.assertion")

local Fmt = require("lunest.inspect.Fmt")
local F = Fmt.new

---@alias lunest.inspect.Value
---| lunest.inspect.Value.Single
---| lunest.inspect.Value.Table
---| lunest.inspect.Value.Ref

---@class lunest.inspect.Value.Base
local Base = {}
---@class lunest.inspect.Value.Single: lunest.inspect.Value.Base
---@field package inner number | boolean | string | function | thread | userdata
local Single = setmetatable({}, Base)
---@class lunest.inspect.Value.Table: lunest.inspect.Value.Base
---@field package pairs { key: lunest.inspect.Value, value: lunest.inspect.Value }[]
local Table = setmetatable({}, Base)
---@class lunest.inspect.Value.Ref: lunest.inspect.Value.Base
---@field package path lunest.inspect.Value[]
local Ref = setmetatable({}, Base)

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
    local function sort(list, value)
        table.sort(list, function(a, b)
            if value then
                a, b = value(a), value(b)
            end
            local ta, tb = type(a), type(b)
            if ta ~= tb then
                a, b = TYPE_ORDERS[ta], TYPE_ORDERS[tb]
            elseif ta ~= "number" and ta ~= "string" then
                a, b = ("%p"):format(a), ("%p"):format(b)
            end
            return a < b
        end)
    end

    ---@param obj any
    ---@param current_ref lunest.inspect.Value.Ref
    ---@param visited_refs table<any, lunest.inspect.Value.Ref>
    ---@return lunest.inspect.Value
    local function new(obj, current_ref, visited_refs)
        if visited_refs[obj] then
            return visited_refs[obj]
        end
        local ty = type(obj)
        if ty == "table" or ty == "function" or ty == "thread" or ty == "userdata" then
            visited_refs[obj] = current_ref:copy()
        end
        if ty ~= "table" then
            return Single.new(obj)
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
            local key = new(e.key, Ref.EMPTY, setmetatable({}, { __index = visited_refs }))
            current_ref:push(key)
            local value = new(e.value, current_ref, visited_refs)
            current_ref:pop()
            e.key, e.value = key, value
        end
        ---@cast list { key: lunest.inspect.Value, value: lunest.inspect.Value }[]
        return Table.new(list)
    end

    ---@return lunest.inspect.Value
    function M.new(any)
        return new(any, Ref.new(), {})
    end

    test.group("new", function()
        test.test("list", function()
            assertion.eq(
                Table.new({
                    { key = Single.new(1), value = Single.new(1) },
                    { key = Single.new(2), value = Single.new(2) },
                    { key = Single.new(4), value = Single.new("a") },
                }),
                M.new({ 1, 2, nil, "a" })
            )
        end)

        test.test("reference", function()
            local t = {}
            t.M = {}
            t.M.__index = t.M
            assertion.eq(
                Table.new({
                    {
                        key = Single.new("M"),
                        value = Table.new({
                            { key = Single.new("__index"), value = Ref.new(Single.new("M")) },
                        }),
                    },
                }),
                M.new(t)
            )
        end)
    end)
end

do
    ---@private
    Base.__index = Base

    ---@return lunest.inspect.Fmt.Entry
    function Base:fmt()
        return nil ---@diagnostic disable-line: return-type-mismatch
    end

    ---@return lunest.inspect.Fmt.Entry
    function Base:fmt_tblkey()
        return nil ---@diagnostic disable-line: return-type-mismatch
    end

    ---@return lunest.inspect.Fmt
    function Base:fmt_wrap()
        return F(self:fmt())
    end

    ---@return lunest.inspect.Fmt.Entry
    function Base:fmt_accessor()
        return self:fmt_tblkey()
    end
end

do
    ---@private
    Single.__index = Single

    ---@param inner number | boolean | string | function | thread | userdata
    ---@return self
    function Single.new(inner)
        local self = setmetatable({}, Single)
        self.inner = inner
        return self
    end

    ---@return lunest.inspect.Fmt.Entry
    function Single:fmt()
        local a = self.inner
        local ty = type(a)
        if ty == "boolean" or ty == "number" then
            return tostring(a)
        elseif ty == "string" then
            return (("%q"):format(a):gsub("\\\n", "\\n"))
        elseif ty == "function" then
            local info = debug.getinfo(a, "S")
            if info.what ~= "Lua" or info.short_src == "" then
                return ("(func .%s %q)"):format(("%p"):format(a):sub(2), info.what)
            end
            local s = info.short_src
            if 0 < (info.linedefined or 0) then
                s = s .. ":" .. info.linedefined
            end
            return ("(func .%s %q)"):format(("%p"):format(a):sub(2), s)
        else
            return ("(%s .%s)"):format(ty, ("%p"):format(a):sub(2))
        end
    end

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
    local function is_ident(s)
        return not KEYWORDS[s] and s:match("^[_%a][_%w]*$") ~= nil
    end

    ---@return lunest.inspect.Fmt.Entry
    function Single:fmt_tblkey()
        local a = self.inner
        if type(a) == "string" and is_ident(a) then
            return a
        else
            return F("[", self:fmt(), "]")
        end
    end

    ---@return lunest.inspect.Fmt.Entry
    function Single:fmt_accessor()
        local a = self.inner
        if type(a) == "string" and is_ident(a) then
            return "." .. a
        else
            return F("[", self:fmt(), "]")
        end
    end

    test.test("Single:fmt_accessor", function()
        assertion.eq(F("[", "1", "]"), Single.new(1):fmt_accessor())
        assertion.eq(".hello", Single.new("hello"):fmt_accessor())
        assertion.eq(F("[", '"end"', "]"), Single.new("end"):fmt_accessor())
    end)
end

do
    ---@private
    Table.__index = Table

    ---@param t { key: lunest.inspect.Value, value: lunest.inspect.Value }[]
    function Table.new(t)
        local self = setmetatable({}, Table)
        self.pairs = t
        return self
    end

    ---@return lunest.inspect.Fmt.Entry
    function Table:fmt()
        local t = self.pairs
        if t[1] == nil then
            return "{}"
        end
        local t_len = #t
        local fmt = F("{", Fmt.Sequence.Indent)
        local prev_key = nil
        for i, e in ipairs(t) do
            local is_last = i == t_len
            prev_key = e.key.inner == (prev_key or 0) + 1 and e.key.inner or nil
            fmt:extend(Fmt.Sequence.NewLineOrSpace)
                :extend(
                    F()
                        :extend_if(not prev_key, e.key:fmt_tblkey(), " = ")
                        :extend(e.value:fmt())
                        :extend_if(not is_last, ",")
                )
                :extend_if(is_last, Fmt.Sequence.TrailingComma)
        end
        return fmt:extend(Fmt.Sequence.Dedent, Fmt.Sequence.NewLineOrSpace, "}")
    end

    ---@return lunest.inspect.Fmt
    function Table:fmt_tblkey()
        return F("[", self:fmt(), "]")
    end

    test.group("table", function()
        test.test("empty", function()
            assertion.eq("{}", Table.new({}):fmt())
        end)

        test.test("list", function()
            assertion.eq(
                F(
                    "{",
                    Fmt.Sequence.Indent,
                    Fmt.Sequence.NewLineOrSpace,
                    F("1", ","),
                    Fmt.Sequence.NewLineOrSpace,
                    F("2"),
                    Fmt.Sequence.TrailingComma,
                    Fmt.Sequence.Dedent,
                    Fmt.Sequence.NewLineOrSpace,
                    "}"
                ),
                M.new({ 1, 2 }):fmt()
            )
        end)

        test.test("dict", function()
            assertion.eq(
                F(
                    "{",
                    Fmt.Sequence.Indent,
                    Fmt.Sequence.NewLineOrSpace,
                    F("a", " = ", "2", ","),
                    Fmt.Sequence.NewLineOrSpace,
                    F("b", " = ", "1"),
                    Fmt.Sequence.TrailingComma,
                    Fmt.Sequence.Dedent,
                    Fmt.Sequence.NewLineOrSpace,
                    "}"
                ),
                M.new({ a = 2, b = 1 }):fmt()
            )
        end)

        test.test("key", function()
            assertion.eq(
                F(
                    "{",
                    Fmt.Sequence.Indent,
                    Fmt.Sequence.NewLineOrSpace,
                    F(
                        F(
                            "[",
                            F(
                                "{",
                                Fmt.Sequence.Indent,
                                Fmt.Sequence.NewLineOrSpace,
                                F("a", " = ", "1"),
                                Fmt.Sequence.TrailingComma,
                                Fmt.Sequence.Dedent,
                                Fmt.Sequence.NewLineOrSpace,
                                "}"
                            ),
                            "]"
                        ),
                        " = ",
                        "2"
                    ),
                    Fmt.Sequence.TrailingComma,
                    Fmt.Sequence.Dedent,
                    Fmt.Sequence.NewLineOrSpace,
                    "}"
                ),
                M.new({ [{ a = 1 }] = 2 }):fmt()
            )
        end)
    end)
end

do
    ---@private
    Ref.__index = Ref

    ---@param ... lunest.inspect.Value
    ---@return self
    function Ref.new(...)
        local self = setmetatable({}, Ref)
        self.path = { ... }
        return self
    end
    Ref.EMPTY = Ref.new()

    ---@param v lunest.inspect.Value
    function Ref:push(v)
        table.insert(self.path, v)
    end

    function Ref:pop()
        assert(table.remove(self.path))
    end

    ---@return self
    function Ref:copy()
        local copied = Ref.new()
        for i, value in ipairs(self.path) do
            copied.path[i] = value
        end
        return copied
    end

    local ROOT_VALUE = "(root)"

    ---@return lunest.inspect.Fmt.Entry
    function Ref:fmt()
        local fmt = F(ROOT_VALUE, Fmt.Sequence.Indent)
        for _, v in ipairs(self.path) do
            fmt:extend(Fmt.Sequence.NewLine):extend(v:fmt_accessor())
        end
        return fmt:extend(Fmt.Sequence.Dedent)
    end

    ---@return lunest.inspect.Fmt
    function Ref:fmt_tblkey()
        return F("[", self:fmt(), "]")
    end

    test.test("Ref:fmt", function()
        assertion.eq(
            F(
                ROOT_VALUE,
                Fmt.Sequence.Indent,
                Fmt.Sequence.NewLine,
                F("[", "0", "]"),
                Fmt.Sequence.NewLine,
                ".hello",
                Fmt.Sequence.NewLine,
                ".world",
                Fmt.Sequence.NewLine,
                F("[", '"!"', "]"),
                Fmt.Sequence.NewLine,
                "._",
                Fmt.Sequence.Dedent
            ),
            Ref.new(
                Single.new(0),
                Single.new("hello"),
                Single.new("world"),
                Single.new("!"),
                Single.new("_")
            ):fmt()
        )
    end)
end

return M
