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

---@param func fun()
function M:run(func)
    local function wrapped()
        current = self
        func()
        for _, f in ipairs(self.deferred) do
            f()
        end
        self.deferred = {}
        current = self.parent
    end
    if self.parent then
        self.parent:defer(wrapped)
    else
        wrapped()
    end
end

---@param func fun()
function M:defer(func)
    table.insert(self.deferred, func)
end

return M
