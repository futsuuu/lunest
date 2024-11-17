---@class lunest.assertion
local M = {}

local test = require("lunest.wrapper")

local inspect = require("lunest.inspect")

---@param left any
---@param right any
---@param left_refs table<table, table>?
---@param right_refs table<table, table>?
---@return boolean
local function equal(left, right, left_refs, right_refs)
    if left == right then
        return true
    end
    if type(left) ~= type(right) then
        return false
    end

    if type(left) == "table" then
        left_refs = left_refs or {}
        right_refs = right_refs or {}

        local id = {}
        left_refs[left] = id
        right_refs[right] = id

        local function eq(left_value, right_value)
            return left_refs[left_value] and left_refs[left_value] == right_refs[right_value]
                or equal(left_value, right_value, left_refs, right_refs)
        end

        for key, left_value in pairs(left) do
            if not eq(left_value, right[key]) then
                return false
            end
        end
        for key, right_value in pairs(right) do
            if not eq(left[key], right_value) then
                return false
            end
        end

        return true
    end

    if type(left) == "function" then
        if not equal(debug.getinfo(left, "Snu"), debug.getinfo(right, "Snu")) then
            return false
        end

        local success, chunk_left, chunk_right
        success, chunk_left = pcall(string.dump, left)
        if not success then
            return false
        end
        success, chunk_right = pcall(string.dump, right)
        if not success then
            return false
        end

        return chunk_left == chunk_right
    end

    return false
end

test.group("eq", function()
    test.group("table", function()
        test.test("empty", function()
            assert(equal({}, {}))
        end)

        test.test("recursive", function()
            local a = {}
            a[1] = a
            local b = {}
            b[1] = b
            assert(equal(a, b))
        end)
    end)

    test.test("function", function()
        local function ret()
            return function()
                print("hello")
            end
        end
        assert(equal(ret(), ret()))
        local function ret2()
            return function()
                print("hello")
            end
        end
        assert(not equal(ret(), ret2()))
    end)
end)

---@param left any
---@param right any
function M.eq(left, right)
    if not equal(left, right) then
        error(
            "two values are not equal\n"
                .. (" left: %s\n"):format(inspect(left))
                .. ("right: %s"):format(inspect(right))
        )
    end
end

---@param left any
---@param right any
function M.ne(left, right)
    assert(not equal(left, right))
end

return M
