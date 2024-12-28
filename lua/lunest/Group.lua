---@class lunest.Group
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

---@param name string
---@param source string
---@return self
function M.new(name, source)
    local self = setmetatable({}, M)
    self.name = name
    self.source = source
    self.parent = current
    self.deferred = {}
    if self.parent and self.parent:is_toplevel() then
        self.parent:defer(function()
            current = self
        end)
    else
        current = self
    end
    return self
end

function M:finish()
    local function inner()
        current = self.parent
    end
    if self.parent and self.parent:is_toplevel() then
        self.parent:defer(inner)
    else
        inner()
    end
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
