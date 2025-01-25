-- NOTE: **DO NOT** import this module directly from the other modules using `require()`.

local Context = require("lunest.Context")
local Group = require("lunest.Group")
local Process = require("lunest.Process")
local Test = require("lunest.Test")
local assertion = require("lunest.assertion")

local function main()
    local process = Process.open(assert(os.getenv("LUNEST_IN")), assert(os.getenv("LUNEST_OUT")))
    local cx = Context.new(process)

    do
        local M = {}
        package.loaded.lunest = M

        M.assertion = assertion

        ---@param name string
        ---@param func fun()
        function M.test(name, func)
            Test.new(cx, name, debug.getinfo(2, "S").source:gsub("^@", ""), func)
        end

        ---@param name string
        ---@param func fun()
        function M.group(name, func)
            Group.new(cx, name, debug.getinfo(2, "S").source:gsub("^@", ""), func)
        end
    end

    process:on_execute(function(script)
        dofile(script)
    end)

    process:on_run(function()
        for _, file in ipairs(cx:target_files()) do
            Group.run_file(cx, file.name, file.path)
        end
    end)

    process:loop()
end

if _G.arg[0] and ("@" .. _G.arg[0]) == debug.getinfo(1, "S").source then
    main()
end
