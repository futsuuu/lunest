local package = {}
do
    ---@type table<string, boolean>
    local PUBLIC_MODULES

    local function copy(dist, tbl)
        for k, v in pairs(tbl) do
            dist[k] = v
        end
        return dist
    end

    package = copy({}, _G.package)
    local package_private = {}

    for _, field in ipairs({ "loaded", "preload" }) do
        package_private[field] = setmetatable({}, { __index = _G.package[field] })

        package[field] = setmetatable({}, {
            __index = package_private[field],
            __newindex = function(_, key, value)
                if PUBLIC_MODULES[key] then
                    _G.package[field][key] = value
                else
                    package_private[field][key] = value
                end
            end,
            __pairs = function()
                local t = {}
                copy(t, _G.package[field])
                copy(t, package_private[field])
                return pairs(t) ---@diagnostic disable-line: redundant-return-value
            end,
        })
    end
end

local pairs = pairs ---@diagnostic disable-line: unused-local
if _VERSION == "Lua 5.1" then
    function pairs(t) ---@diagnostic disable-line: unused-function, unused-local
        if t == package.loaded or t == package.preload then
            return getmetatable(t).__pairs(t)
        end
        return _G.pairs(t)
    end
end

---@param modname string
local function require(modname) ---@diagnostic disable-line: unused-function, unused-local
    if package.loaded[modname] ~= nil then
        return package.loaded[modname]
    end

    if package.preload[modname] then
        local mod = package.preload[modname](modname)

        -- module was manually set to `package.loaded` by `package.preload[modname]`
        if package.loaded[modname] ~= nil then
            return package.loaded[modname]
        end

        if mod == nil then
            package.loaded[modname] = true
        else
            package.loaded[modname] = mod
        end
        return mod
    end

    return _G.require(modname)
end
