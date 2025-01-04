local M = {}

local test = require("lunest.wrapper")

local bridge = require("lunest.bridge")

---@generic F: function
---@param func F
---@return F
function M.isolated(func)
    return function(...)
        local modules = {} ---@type table<string, true>
        for key, _ in pairs(package.loaded) do
            modules[key] = true
        end
        return (function(...)
            for key, _ in pairs(package.loaded) do
                if not modules[key] then
                    package.loaded[key] = nil
                end
            end
            return ...
        end)(func(...))
    end
end

test.test("isolated", function()
    M.isolated(function()
        package.loaded["test.foo"] = 1

        M.isolated(function()
            package.loaded["test.bar"] = 2
        end)()

        assert(package.loaded["test.foo"] == 1)
        assert(package.loaded["test.bar"] == nil)
    end)()

    assert(package.loaded["test.foo"] == nil)
end)

---@param path string
---@param cwd string
---@return string
local function normalize_path(path, cwd)
    local sep = path:match("[/\\]") or cwd:match("[/\\]") or package.config:sub(1, 1)
    if not (path:match("^[/\\]") or path:match("^%a:[/\\]")) then
        path = cwd .. sep .. path
    end
    path = path:gsub("([/\\])%.[/\\]", "%1"):gsub("[^/\\]+[/\\]%.%.[/\\]?", "")
    return path
end

test.test("normalize_path", function()
    assert(normalize_path("./foo", "/bar") == "/bar/foo")
    assert(normalize_path("/foo/./bar", "/") == "/foo/bar")
    assert(normalize_path("/foo/../bar", "/") == "/bar")
end)

---@param file string
---@param cwd string
---@param path string
---@return string?
local function name(file, cwd, path)
    local templates = {}
    for t in path:gmatch("[^;]+") do
        table.insert(templates, 1, normalize_path(t, cwd))
    end

    for _, template in ipairs(templates) do
        local prefix, suffix = template:match("([^?]*)?(.*)") ---@type string, string
        if
            file:find(prefix, 1, true) == 1
            and select(2, file:find(suffix, 1, true)) == file:len()
        then
            return (file:sub(prefix:len() + 1, file:len() - suffix:len()):gsub("[/\\]", "."))
        end
    end
end

test.test("name", function()
    local r = "/cwd"
    local t = "./?.lua;./?/init.lua;/share/lua/?.lua"
    assert(name("/cwd/foo/bar.lua", r, t) == "foo.bar")
    assert(name("/cwd/foo/bar/init.lua", r, t) == "foo.bar")
    assert(name("/share/lua/foo/init.lua", r, t) == "foo.init")
end)

function M.name(file)
    return name(file, bridge.root_dir(), package.path)
end

return M
