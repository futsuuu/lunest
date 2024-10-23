local M = {}

---@type { name: string, path: string }[]
local TARGET_FILES
---@type string
local MSG_FILE
---@type string?
local INIT_FILE

---@class lunest.bridge.Message
---@field TestStarted? lunest.bridge.TestStarted
---@field TestFinished? lunest.bridge.TestFinished

---@class lunest.bridge.TestStarted
---@field title string[]

---@class lunest.bridge.TestFinished
---@field title string[]
---@field error lunest.bridge.TestError?

---@class lunest.bridge.TestError
---@field Msg? string

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

---@type file*
local msg_file

---@param message lunest.bridge.Message
local function write_msg(message)
    if not msg_file then
        msg_file = assert(io.open(MSG_FILE, "a"))
    end
    assert(msg_file:write(json_encode(message) .. "\n"))
    msg_file:flush()
end

---@param title string[]
function M.start_test(title)
    write_msg({
        TestStarted = { title = title },
    })
end

---@param title string[]
---@param err string?
function M.finish_test(title, err)
    ---@type lunest.bridge.Message
    local msg = {
        TestFinished = { title = title },
    }
    if err then
        msg.TestFinished.error = { Msg = err }
    end
    write_msg(msg)
end

function M.finish()
    if msg_file then
        assert(msg_file:close())
    end
end

return M
