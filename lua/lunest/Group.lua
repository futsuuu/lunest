---@class lunest.Group
---@field name string
---@field source string
---@field parent lunest.Group?
---@field deferred fun()[]
local M = {}

---@private
M.__index = M

---@type lunest.Group?
M.current = nil

---@param name string
---@param source string
---@return self
function M.new(name, source)
    ---@type lunest.Group
    local self = {
        name = name,
        source = source,
        parent = M.current,
        deferred = {},
    }
    setmetatable(self, M)
    M.current = self
    return self
end

function M:finish()
    M.current = self.parent
end

---@return boolean
function M:is_toplevel()
    return self.parent == nil
end

---@param func fun()
function M:run(func)
    if self.parent and self.parent.source ~= self.source then
        return
    end
    if self.parent and self.parent:is_toplevel() then
        self.parent:defer(func)
    else
        func()
    end
    for _, f in ipairs(self.deferred) do
        f()
    end
    self.deferred = {}
end

---@param func fun()
function M:defer(func)
    table.insert(self.deferred, func)
end

return M
