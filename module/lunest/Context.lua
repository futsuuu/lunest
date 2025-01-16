---@class lunest.Context
---@field package _process lunest.Process
---@field package _mode "Run" | "List"
---@field package _root_dir string
---@field package _term_width integer
local M = {}
---@private
M.__index = M

---@param process lunest.Process
---@return self
function M.new(process)
    local self = setmetatable({}, M)
    self._process = process

    process:on_initialize(function(input)
        self._mode = input.mode
        self._root_dir = input.root_dir
        self._term_width = input.term_width
    end)

    return self
end

---@return lunest.Process
function M:process()
    return self._process
end

---@return "Run" | "List"
function M:mode()
    return self._mode
end

---@return string
function M:root_dir()
    return self._root_dir
end

---@return integer
function M:term_width()
    return self._term_width
end

return M
