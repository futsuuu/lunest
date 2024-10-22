local M = {}

---@type { name: string, path: string }[]
local TARGET_FILES
---@type string
local RESULT_DIR
---@type string?
local INIT_FILE

---@return { name: string, path: string }[]
function M.get_target_files()
    return TARGET_FILES
end

---@return string?
function M.get_init_file()
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
function M.write_result(title, err)
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

return M
