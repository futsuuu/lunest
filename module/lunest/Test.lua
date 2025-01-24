---@class lunest.Test
---@field package cx lunest.Context
---@field package id string
---@field package name string
---@field package func fun()
---@field package source string
---@field package parent lunest.Group
local M = {}

local Group = require("lunest.Group")
local inspect = require("lunest.inspect")

---@type lunest.Test?
local current = nil

---@return lunest.Test?
---@return string?
function M.current()
    if current then
        return current
    else
        return nil, "there is no test currently running"
    end
end

---@private
M.__index = M

---@param cx lunest.Context
---@param name string
---@param source string
---@param func fun()
---@return self?
function M.new(cx, name, source, func)
    local parent = assert(Group.current())
    if parent.source ~= source then
        return
    end
    local self = setmetatable({}, M)
    self.cx = cx
    self.name = name
    self.func = func
    self.source = source
    self.parent = parent
    self.id = self.parent:register(self)
    return self
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
---@param info lunest.TestErrorInfo?
---@param level integer?
function M:error(msg, info, level)
    level = level or 1
    local debuginfo = debug.getinfo(level + 2, "Sl")
    if debuginfo.short_src then
        local src = debuginfo.short_src
        if debuginfo.currentline then
            src = src .. ":" .. debuginfo.currentline
        end
        msg = src .. ": " .. msg
    end
    ---@type lunest.TestError
    local err = {
        message = msg,
        traceback = traceback(level + 1),
        info = info,
    }
    error(setmetatable(err, error_mt))
end

---@param msg string
---@param left any
---@param right any
---@param level integer?
function M:error_with_diff(msg, left, right, level)
    level = level or 1
    local inspect_width = self.cx:term_width() - 1 -- consider diff sign character
    self:error(msg, {
        Diff = {
            left = inspect.inspect(left, inspect_width),
            right = inspect.inspect(right, inspect_width),
        },
    }, level + 1)
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
    ---@type lunest.TestError
    return {
        message = tostring(err),
        traceback = traceback(level + 1),
    }
end

function M:run()
    local title = self:get_title()
    local mode = self.cx:mode()
    if mode == "List" then
        self.cx:process():send_test_info(self.id, title)
    elseif mode == "Run" then
        self.cx:process():notify_test_started(title)
        assert(not current)
        current = self
        local success, err = xpcall(test_runner(self.func), handle_error)
        current = nil
        if success then
            err = nil
        end
        self.cx:process():notify_test_finished(title, err)
    end
end

---@return string[]
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
