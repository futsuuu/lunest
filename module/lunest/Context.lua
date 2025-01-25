---@class lunest.Context
---@field package id_set table<string, true>
---@field package _process lunest.Process
---@field package _mode "run" | "list"
---@field package _root_dir string
---@field package _target_files { name: string, path: string }[]
---@field package _term_width integer
local M = {}

local id = require("lunest.id")

---@private
M.__index = M

---@param process lunest.Process
---@return self
function M.new(process)
    local self = setmetatable({}, M)
    self._process = process

    process:on_initialize(function(input)
        self._root_dir = input.root_dir
        self._target_files = input.target_files
        self._term_width = input.term_width
    end)

    process:on_run_tests(function(input)
        self._mode = "run"
        self.id_set = id.create_set(input.ids)
    end)

    process:on_send_test_info(function()
        self._mode = "list"
    end)

    return self
end

---@return lunest.Process
function M:process()
    return self._process
end

---@param _id string
---@return boolean
function M:is_id_enabled(_id)
    local set = self.id_set
    if not set then
        return true
    end
    return set[_id] == true
end

---@return "run" | "list"
function M:mode()
    return self._mode
end

---@return string
function M:root_dir()
    return self._root_dir
end

---@return { name: string, path: string }[]
function M:target_files()
    return self._target_files
end

---@return integer
function M:term_width()
    return self._term_width
end

return M
