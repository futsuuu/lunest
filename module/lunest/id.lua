local M = {}

local test = require("lunest.wrapper")
local assertion = test.assertion

---@param name string
---@return string
function M.toplevel(name)
    return name .. ":"
end

---@param id string
---@param index integer
---@return string
function M.join(id, index)
    return ("%s/%x"):format(id, index)
end

test.test("join", function()
    assertion.eq("/a:/a/11", M.join(M.join(M.toplevel("/a"), 10), 17))
end)

---@param id_list string[]
---@return table<string, true>
function M.create_set(id_list)
    local set = {}
    for _, id in ipairs(id_list) do
        set[id] = true
        local _, i = id:find(":/", nil, true)
        while i do
            set[id:sub(1, i - 1)] = true
            i = id:find("/", i + 2, true)
        end
    end
    return set
end

test.test("create_set", function()
    assertion.eq(
        {
            ["/a/b:"] = true,
            ["/a/b:/12"] = true,
            ["/a/b:/12/a"] = true,
            ["/a/b:/12/a/11"] = true,
            ["/a/b:/12/b"] = true,
        },
        M.create_set({
            "/a/b:/12/a/11",
            "/a/b:/12/b",
        })
    )
end)

return M
