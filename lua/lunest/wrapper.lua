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

return M
