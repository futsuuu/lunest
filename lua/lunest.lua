-- NOTE: **DO NOT** import this module directly from the other modules using `require()`.
local M = {}

local Group = require("lunest.Group")
local Test = require("lunest.Test")
local bridge = require("lunest.bridge")
local module = require("lunest.module")

---@param name string
---@param func fun()
function M.test(name, func)
    local test = Test.new(name, (debug.getinfo(func, "S").source:gsub("^@", "")))
    test:run(func)
end

---@param name string
---@param func fun()
function M.group(name, func)
    local group = Group.new(name, (debug.getinfo(func, "S").source:gsub("^@", "")))
    group:run(func)
    group:finish()
end

---@param name string
---@param path string
local function run_toplevel_group(name, path)
    local group = Group.new(name, path)
    group:run(module.isolated(function()
        assert(loadfile(path))(module.name(path))
    end))
    group:finish()
end

local function main()
    package.loaded.lunest = M

    local init_file = bridge.get_init_file()
    if init_file then
        dofile(init_file)
    end

    for _, file in ipairs(bridge.get_target_files()) do
        run_toplevel_group(file.name, file.path)
    end

    bridge.finish()
end

if arg[0] == debug.getinfo(1, "S").source:gsub("^@", "") then
    main()
end
