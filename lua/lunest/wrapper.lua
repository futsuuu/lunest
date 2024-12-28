if package.loaded.lunest then
    return package.loaded.lunest
end

---@meta

---@class lunest
local M = {}

---@param name string
---@param func fun()
function M.test(name, func) end

---@param name string
---@param func fun()
function M.group(name, func) end

---@class lunest.assertion
local assertion = {}
M.assertion = assertion

---@param left any
---@param right any
function assertion.eq(left, right) end

---@param left any
---@param right any
function assertion.ne(left, right) end

return M
