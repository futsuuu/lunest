-- NOTE: **DO NOT** import this module directly from the other modules using `require()`.

local Context = require("lunest.Context")
local Group = require("lunest.Group")
local Process = require("lunest.Process")
local Test = require("lunest.Test")
local assertion = require("lunest.assertion")
local module = require("lunest.module")

local function main()
    local process = Process.open()
    local cx = Context.new(process)

    do
        local M = {}
        package.loaded.lunest = M

        M.assertion = assertion

        ---@param name string
        ---@param func fun()
        function M.test(name, func)
            local test = Test.new(cx, name, (debug.getinfo(func, "S").source:gsub("^@", "")))
            test:run(func)
        end

        ---@param name string
        ---@param func fun()
        function M.group(name, func)
            local group = Group.new(cx, name, (debug.getinfo(func, "S").source:gsub("^@", "")))
            group:run(func)
            group:finish()
        end
    end

    process:on_initialize(function(input)
        local init_file = input.init_file
        if init_file then
            dofile(init_file)
        end

        for _, file in ipairs(input.target_files) do
            local group = Group.new(cx, file.name, file.path)
            group:run(module.isolated(function()
                assert(loadfile(file.path))(module.name(cx:root_dir(), file.path))
            end))
            group:finish()
        end

        process:close()
    end)

    process:loop()
end

if arg[0] == debug.getinfo(1, "S").source:gsub("^@", "") then
    main()
end
