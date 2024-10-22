local M = {}

---@class lunest.Test
---@field name string
---@field source string
---@field parent lunest.Group
local Test = {}

---@class lunest.Group
---@field name string
---@field source string
---@field parent lunest.Group?
---@field deferred fun()[]
local Group = {}

local bridge = {}

local main
do
    ---@param name string
    ---@param path string
    local function run_toplevel_group(name, path)
        local group = Group.new(name, path)

        group:run(function()
            local modules = {} ---@type table<string, true>
            for key, _ in pairs(package.loaded) do
                modules[key] = true
            end

            dofile(path)

            for key, _ in pairs(package.loaded) do
                if not modules[key] then
                    package.loaded[key] = nil
                end
            end
        end)

        group:finish()
    end

    function main()
        package.loaded.lunest = setmetatable({}, M)

        local init_file = bridge.get_init_file()
        if init_file then
            dofile(init_file)
        end

        for _, file in ipairs(bridge.get_target_files()) do
            run_toplevel_group(file.name, file.path)
        end
    end
end

do
    ---@private
    M.__index = M

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
end

do
    ---@private
    Test.__index = Test

    ---@param name string
    ---@param source string
    ---@return self
    function Test.new(name, source)
        ---@type lunest.Test
        local self = {
            name = name,
            source = source,
            parent = assert(Group.current),
        }
        return setmetatable(self, Test)
    end

    ---@param func fun()
    function Test:run(func)
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
    function Test:wrap(func)
        return function()
            local success, err = pcall(func)
            if success then
                err = nil
            else
                err = err and tostring(err) or "error occurred without message"
            end
            bridge.write_result(self:get_title(), err)
        end
    end

    function Test:get_title()
        local title = { self.name }
        local group = self.parent
        repeat
            table.insert(title, 1, group.name)
            group = group.parent
        until not group
        return title
    end
end

do
    ---@private
    Group.__index = Group

    ---@type lunest.Group?
    Group.current = nil

    ---@param name string
    ---@param source string
    ---@return self
    function Group.new(name, source)
        ---@type lunest.Group
        local self = {
            name = name,
            source = source,
            parent = Group.current,
            deferred = {},
        }
        setmetatable(self, Group)
        Group.current = self
        return self
    end

    function Group:finish()
        Group.current = self.parent
    end

    ---@return boolean
    function Group:is_toplevel()
        return self.parent == nil
    end

    ---@param func fun()
    function Group:run(func)
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
    function Group:defer(func)
        table.insert(self.deferred, func)
    end
end

do
    ---@type { name: string, path: string }[]
    local TARGET_FILES
    ---@type string
    local RESULT_DIR
    ---@type string?
    local INIT_FILE

    ---@return { name: string, path: string }[]
    function bridge.get_target_files()
        return TARGET_FILES
    end

    ---@return string?
    function bridge.get_init_file()
        return INIT_FILE
    end

    local null = {}

    ---@return string
    local function json_encode(obj)
        local s
        local t = type(obj)
        if t == "nil" or obj == null then
            s = "null"
        elseif t == "number" or t == "boolean" then
            s = tostring(obj)
        elseif t == "string" then
            s = ("%q"):format(obj)
        elseif t ~= "table" then
            error(("invalid type '%s'"):format(t))
        elseif obj[1] then
            local ss = {}
            for _, value in ipairs(obj) do
                table.insert(ss, json_encode(value))
            end
            s = "[" .. table.concat(ss, ",") .. "]"
        else
            local ss = {}
            for key, value in pairs(obj) do
                table.insert(ss, ("%q:%s"):format(key, json_encode(value)))
            end
            s = "{" .. table.concat(ss, ",") .. "}"
        end
        return (s:gsub("\\\n", "\\n"))
    end

    local counter = 0

    ---@param title string[]
    ---@param err string?
    function bridge.write_result(title, err)
        local result = {
            title = title,
        }
        if err then
            result.error = { Msg = err }
        end
        local file = assert(io.open(("%s/%x.json"):format(RESULT_DIR, counter), "w"))
        assert(file:write(json_encode(result)))
        assert(file:close())
        counter = counter + 1
    end
end

if arg[0] == debug.getinfo(1, "S").source:gsub("^@", "") then
    main()
end
