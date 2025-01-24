---@class lunest.Group
---@field cx lunest.Context
---@field id string
---@field name string
---@field func fun()
---@field source string
---@field parent lunest.Group?
---@field children (lunest.Test | lunest.Group)[]
local M = {}

local id = require("lunest.id")
local module = require("lunest.module")

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
---@param path string
function M.run_file(cx, name, path)
    M.new(
        cx,
        name,
        path,
        module.isolated(function()
            assert(loadfile(path))(module.name(cx:root_dir(), path))
        end)
    ):run()
end

---@param cx lunest.Context
---@param name string
---@param source string
---@param func fun()
---@return self?
function M.new(cx, name, source, func)
    if current and current.source ~= source then
        return
    end
    local self = setmetatable({}, M)
    self.cx = cx
    self.name = name
    self.func = func
    self.source = source
    self.parent = current
    self.children = {}
    if current then
        self.id = current:register(self)
    else
        self.id = id.toplevel(name)
    end
    return self
end

function M:run()
    current = self
    self.func()
    for _, child in ipairs(self.children) do
        child:run()
    end
    current = self.parent
end

---@param child (lunest.Group | lunest.Test)[]
---@return string
function M:register(child)
    local i = #self.children + 1
    self.children[i] = child
    return id.join(self.id, i)
end

return M
