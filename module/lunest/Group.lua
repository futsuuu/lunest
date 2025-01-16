---@class lunest.Group
---@field cx lunest.Context
---@field name string
---@field source string
---@field parent lunest.Group?
---@field deferred fun()[]
local M = {}

---@type lunest.Group?
local current = nil

---@return lunest.Group?
function M.current()
    return current
end

---@private
M.__index = M

---@param cx lunest.Context
---@param name string
---@param source string
---@return self
function M.new_toplevel(cx, name, source)
    return assert(M.new(cx, name, source))
end

---@param cx lunest.Context
---@param name string
---@param source string
---@return self?
function M.new(cx, name, source)
    if current and current.source ~= source then
        return
    end
    local self = setmetatable({}, M)
    self.cx = cx
    self.name = name
    self.source = source
    self.parent = current
    self.deferred = {}
    return self
end

---@return boolean
function M:is_toplevel()
    return self.parent == nil
end

---@param func fun()
function M:run(func)
    func = self:wrap(func)
    if self.parent and self.parent:is_toplevel() then
        self.parent:defer(func)
    else
        func()
    end
end

---@param func fun()
---@return fun()
function M:wrap(func)
    return function()
        current = self
        func()
        for _, f in ipairs(self.deferred) do
            f()
        end
        self.deferred = {}
        current = self.parent
    end
end

---@param func fun()
function M:defer(func)
    table.insert(self.deferred, func)
end

return M
