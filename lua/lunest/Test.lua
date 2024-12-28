---@class lunest.Test
---@field name string
---@field source string
---@field parent lunest.Group
local M = {}

local Group = require("lunest.Group")
local bridge = require("lunest.bridge")

---@private
M.__index = M

---@param name string
---@param source string
---@return self
function M.new(name, source)
    local self = setmetatable({}, M)
    self.name = name
    self.source = source
    self.parent = assert(Group.current())
    return self
end

---@param func fun()
function M:run(func)
    if self.parent.source ~= self.source then
        return
    end
    func = self:wrap(func)
    if self.parent:is_toplevel() then
        self.parent:defer(func)
    else
        func()
    end
end

---@param func fun()
function M:wrap(func)
    local title = self:get_title()
    return function()
        bridge.start_test(title)
        local success, err = pcall(func)
        if success then
            err = nil
        end
        bridge.finish_test(title, err)
    end
end

function M:get_title()
    local title = { self.name }
    local group = self.parent
    repeat
        table.insert(title, 1, group.name)
        group = group.parent
    until not group
    return title
end

return M
