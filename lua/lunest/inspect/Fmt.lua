---@class lunest.inspect.Fmt
---@field package max_expand_lv uinteger
---@field package has_newline_sequence boolean
---@field package list lunest.inspect.Fmt.Entry[]
---@field package mutable boolean
local M = {}
package.loaded[...] = M

local test = require("lunest.wrapper")

local assertion = require("lunest.assertion")

---@enum lunest.inspect.Fmt.Sequence
local Sequence = {
    NewLine = 0,
    NewLineOrSpace = 1,
    Indent = 2,
    Dedent = 3,
    TrailingComma = 4,
}
M.Sequence = Sequence

---@alias lunest.inspect.Fmt.Entry
---| lunest.inspect.Fmt
---| lunest.inspect.Fmt.Sequence
---| string

---@private
M.__index = M

---@param ... lunest.inspect.Fmt.Entry
---@return self
function M.new(...)
    local self = setmetatable({}, M)
    self.max_expand_lv = 0
    self.has_newline_sequence = false
    self.list = {}
    self.mutable = true
    return self:extend(...)
end
local F = M.new

---@param ... lunest.inspect.Fmt.Entry
---@return self
function M:extend(...)
    assert(self.mutable, "cannot mutate after being passed to another 'lunest.inspect.Format'")
    for _, child in ipairs({ ... }) do
        table.insert(self.list, child)
        if child == Sequence.NewLine or child == Sequence.NewLineOrSpace then
            self.has_newline_sequence = true
        elseif type(child) == "table" then
            child.mutable = false
            if child:is_expandable() then
                self.max_expand_lv = self.max_expand_lv + 1
            end
        end
    end
    return self
end

---@param cond any
---@param ... lunest.inspect.Fmt.Entry
---@return self
function M:extend_if(cond, ...)
    return cond and self:extend(...) or self
end

---@return fun(): integer?, lunest.inspect.Fmt.Entry?
function M:iter_rev()
    local index = #self.list + 1
    return function()
        index = index - 1
        local entry = self.list[index]
        if entry then
            return index, entry
        end
    end
end

---@return boolean
function M:is_expandable()
    return self.has_newline_sequence or 0 < self.max_expand_lv
end

---@return string
function M:display_folded()
    local ln = ""
    for _, child in self:iter_rev() do
        local ty = type(child)
        if ty == "string" then
            ln = child .. ln
        elseif ty == "table" then
            ln = child:display_folded() .. ln
        elseif child == Sequence.NewLineOrSpace then
            ln = " " .. ln
        end
    end
    return ln
end

test.group("display_folded", function()
    test.test("string", function()
        assertion.eq("abc", F("a", "b", "c"):display_folded())
    end)

    test.test("sequence", function()
        assertion.eq(
            "a bc",
            F("a", Sequence.NewLineOrSpace, "b", Sequence.NewLine, "c"):display_folded()
        )
    end)

    test.test("nested", function()
        assertion.eq("abcde", F("a", F("b"), "c", F("d", F("e"))):display_folded())
    end)
end)

local INDENT = "  "
local INDENT_WIDTH = #INDENT

---@param suffix_len integer
---@param first_line string
---@param remaining_lines string?
local function first_line_len(suffix_len, first_line, remaining_lines)
    return #first_line + ((remaining_lines == nil or remaining_lines == "") and suffix_len or 0)
end

---@param max_width integer
---@param indent_lv integer
---@param suffix_len integer
---@param first_line string
---@param remaining_lines string?
local function max_width_over(max_width, indent_lv, suffix_len, first_line, remaining_lines)
    return max_width
        < first_line_len(suffix_len, first_line, remaining_lines) + INDENT_WIDTH * indent_lv
end

---@param max_width integer
---@param indent_lv integer
---@param suffix_len integer
---@return string? first_line
function M:try_display_folded(max_width, indent_lv, suffix_len)
    local ln = ""
    for _, child in self:iter_rev() do
        local ty = type(child)
        if ty == "string" then
            ln = child .. ln
        elseif ty == "table" then
            local ret = child:try_display_folded(max_width, indent_lv, #ln)
            if not ret then
                return
            end
            ln = ret .. ln
        elseif child == Sequence.NewLineOrSpace then
            ln = " " .. ln
        end
        if max_width_over(max_width, indent_lv, suffix_len, ln) then
            return
        end
    end
    return ln
end

---@param max_width integer
---@param indent_lv integer
---@param suffix_len integer
---@return string first_line
---@return string remaining_lines
function M:display_expanded(max_width, indent_lv, suffix_len)
    local ln, s = "", ""
    for _, child in self:iter_rev() do
        local ty = type(child)
        if ty == "string" then
            ln = child .. ln
        elseif ty == "table" then
            local first, remaining =
                child:display_auto(max_width, indent_lv, first_line_len(suffix_len, ln, s))
            if remaining == "" then
                ln = first .. ln
            else
                s = remaining .. ln .. s
                ln = first
            end
        elseif child == Sequence.NewLine or child == Sequence.NewLineOrSpace then
            s = "\n" .. INDENT:rep(indent_lv) .. ln .. s
            ln = ""
        elseif child == Sequence.TrailingComma then
            ln = "," .. ln
        elseif child == Sequence.Indent then
            indent_lv = indent_lv - 1
        elseif child == Sequence.Dedent then
            indent_lv = indent_lv + 1
        end
    end
    return ln, s
end

---@param max_width integer
---@param indent_lv integer
---@param suffix_len integer
---@return string first_line
---@return string remaining_lines
function M:display_auto(max_width, indent_lv, suffix_len)
    if not self:is_expandable() then
        return self:display_folded(), ""
    end
    local folded = self:try_display_folded(max_width, indent_lv, suffix_len)
    if folded then
        return folded, ""
    end
    if self.has_newline_sequence or self.max_expand_lv == 0 then
        return self:display_expanded(max_width, indent_lv, suffix_len)
    end
    local ln, s
    for expand_lv = 1, self.max_expand_lv do
        ln, s = "", ""
        local breaked = false
        local expanded = 0
        for _, child in self:iter_rev() do
            local ty = type(child)
            if ty == "string" then
                ln = child .. ln
            elseif ty == "table" then
                if expanded < expand_lv and child:is_expandable() then
                    local first, remaining = child:display_expanded(
                        max_width,
                        indent_lv,
                        first_line_len(suffix_len, ln, s)
                    )
                    if remaining == "" then
                        ln = first .. ln
                    else
                        s = remaining .. ln .. s
                        ln = first
                    end
                    expanded = expanded + 1
                else
                    ln = child:display_folded() .. ln
                end
            elseif child == Sequence.TrailingComma then
                ln = "," .. ln
            else
                error()
            end
            if
                expand_lv ~= self.max_expand_lv
                and expanded == expand_lv
                and max_width_over(max_width, indent_lv, suffix_len, ln, s)
            then
                breaked = true
                break
            end
        end
        if not breaked then
            return ln, s
        end
    end
    return ln, s
end

---@param max_width integer?
---@return string
function M:display(max_width)
    local first_line, remaining_lines = self:display_auto(max_width or math.huge, 0, 0)
    return first_line .. remaining_lines
end

test.group("display", function()
    test.test("concat strings", function()
        assertion.eq("abcde", F("ab", F("c", "d"), "e"):display())
    end)

    test.test("new line", function()
        assertion.eq("ab", F("a", Sequence.NewLine, "b"):display())
        assertion.eq("a\nb", F("a", Sequence.NewLine, "b"):display(0))
        assertion.eq("a b", F("a", Sequence.NewLineOrSpace, "b"):display())
        assertion.eq("a\nb", F("a", Sequence.NewLineOrSpace, "b"):display(0))
    end)

    test.test("indent and dedent", function()
        local fmt =
            F("a", Sequence.Indent, Sequence.NewLine, "b", Sequence.Dedent, Sequence.NewLine, "c")
        assertion.eq(
            table.concat({
                "a",
                "  b",
                "c",
            }, "\n"),
            fmt:display(0)
        )
    end)

    test.test("expand children", function()
        local fmt = F(F("a", Sequence.NewLine), F("b", Sequence.NewLine), "c")
        assertion.eq("abc", fmt:display(3))
        assertion.eq("ab\nc", fmt:display(2))
        assertion.eq("a\nb\nc", fmt:display(1))
    end)
end)

return M
