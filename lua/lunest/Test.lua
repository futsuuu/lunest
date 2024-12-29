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

---@param func function
local function test_runner(func)
    ---@param ___ function
    local function ___LUNEST_TRACEBACK_MARKER(___)
        local r = ___()
        return r -- avoid tail call
    end
    return function()
        local r = ___LUNEST_TRACEBACK_MARKER(func)
        return r
    end
end

---@param level integer?
---@return string
local function traceback(level)
    level = level or 1
    return (
        debug
            .traceback("", level + 1)
            :gsub("^\nstack traceback:\n\t", "")
            :gsub("\n\t", "\n")
            :gsub("\n[^\n]*___LUNEST_TRACEBACK_MARKER.*", "")
    )
end

local error_mt = {}

---@param msg string
---@param info lunest.bridge.TestErrorInfo?
---@param level integer?
function M.error(msg, info, level)
    level = level or 1
    local debuginfo = debug.getinfo(level + 2, "Sl")
    if debuginfo.short_src then
        local src = debuginfo.short_src
        if debuginfo.currentline then
            src = src .. ":" .. debuginfo.currentline
        end
        msg = src .. ": " .. msg
    end
    ---@type lunest.bridge.TestError
    local err = {
        message = msg,
        traceback = traceback(level + 1),
        info = info,
    }
    error(setmetatable(err, error_mt))
end

---@param err any
---@param level integer?
local function handle_error(err, level)
    level = level or 1
    if getmetatable(err) == error_mt then
        return err
    end
    if err == nil then
        err = "(error occurred without message)"
    end
    ---@type lunest.bridge.TestError
    return {
        message = tostring(err),
        traceback = traceback(level + 1),
    }
end

---@param func fun()
function M:wrap(func)
    local title = self:get_title()
    return function()
        bridge.start_test(title)
        local success, err = xpcall(test_runner(func), handle_error)
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
