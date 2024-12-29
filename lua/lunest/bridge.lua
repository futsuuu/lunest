local M = {}

do
    ---@type string
    local ROOT_DIR
    ---@type { name: string, path: string }[]
    local TARGET_FILES
    ---@type string
    local MSG_FILE
    ---@type string?
    local INIT_FILE
    ---@type integer
    local TERM_WIDTH

    ---@return string
    function M.root_dir()
        return ROOT_DIR
    end
    ---@return { name: string, path: string }[]
    function M.get_target_files()
        return TARGET_FILES
    end
    ---@return string
    function M.get_msg_file_path()
        return MSG_FILE
    end
    ---@return string?
    function M.get_init_file()
        return INIT_FILE
    end
    ---@return integer
    function M.get_term_width()
        return TERM_WIDTH
    end
end

local json = {}
do
    ---@param obj any
    ---@return string
    local function format_q(obj)
        return (
            ("%q")
                :format(obj)
                :gsub("\\\n", [[\n]])
                :gsub([[([^\])\8]], [[%1\b]])
                :gsub([[([^\])\9]], [[%1\t]])
                :gsub([[([^\])\12]], [[%1\f]])
                :gsub([[([^\])\13]], [[%1\r]])
        )
    end

    ---@return string
    function json.encode(obj)
        local t = type(obj)
        if t == "nil" then
            return "null"
        elseif t == "number" or t == "boolean" then
            return tostring(obj)
        elseif t == "string" then
            return format_q(obj)
        elseif t ~= "table" then
            error(("invalid type '%s'"):format(t))
        elseif obj[1] then
            local ss = {}
            for _, value in ipairs(obj) do
                table.insert(ss, json.encode(value))
            end
            return "[" .. table.concat(ss, ",") .. "]"
        else
            local ss = {}
            for key, value in pairs(obj) do
                table.insert(ss, format_q(key) .. ":" .. json.encode(value))
            end
            return "{" .. table.concat(ss, ",") .. "}"
        end
    end
end

---@class lunest.bridge.Message
---@field TestStarted? lunest.bridge.TestStarted
---@field TestFinished? lunest.bridge.TestFinished

---@class lunest.bridge.TestStarted
---@field title string[]

---@class lunest.bridge.TestFinished
---@field title string[]
---@field error lunest.bridge.TestError?

---@class lunest.bridge.TestError
---@field message string
---@field traceback string
---@field info lunest.bridge.TestErrorInfo?

---@class lunest.bridge.TestErrorInfo
---@field Diff? { left: string, right: string }

---@type file*
local msg_file

---@param message lunest.bridge.Message
local function write_msg(message)
    if not msg_file then
        msg_file = assert(io.open(M.get_msg_file_path(), "a"))
    end
    assert(msg_file:write(json.encode(message) .. "\n"))
    msg_file:flush()
end

---@param title string[]
function M.start_test(title)
    write_msg({
        TestStarted = { title = title },
    })
end

---@param title string[]
---@param err lunest.bridge.TestError?
function M.finish_test(title, err)
    write_msg({
        TestFinished = {
            title = title,
            error = err,
        },
    })
end

function M.finish()
    if msg_file then
        assert(msg_file:close())
    end
end

return M
