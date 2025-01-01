local M = {}

local json = require("json")

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
