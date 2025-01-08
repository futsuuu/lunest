local M = {}

local json = require("json")

local File = require("lunest.File")

---@return lunest.File
local function open_input_file()
    return File.open(assert(os.getenv("LUNEST_IN")), "r")
end

---@return lunest.File
local function open_output_file()
    return File.open(assert(os.getenv("LUNEST_OUT")), "a")
end

---@alias lunest.bridge.Input
---| lunest.bridge.Initialize

---@class lunest.bridge.Initialize
---@field type "Initialize"
---@field init_file string?
---@field root_dir string
---@field target_files { name: string, path: string }[]
---@field term_width integer

---@class lunest.bridge.Output
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

---@type lunest.File
local input_file
---@type lunest.File
local output_file

local input_reader = coroutine.create(function()
    if not input_file then
        input_file = open_input_file()
    end
    local buf = ""
    while not input_file:is_closed() do
        local line = input_file:readln()
        if line then
            if line:sub(#line) == "\n" then
                coroutine.yield(buf .. line)
                buf = ""
            else
                buf = buf .. line
            end
        end
    end
end)

---@type lunest.bridge.Initialize
local initialize

function M.read_input()
    local _, line = assert(coroutine.resume(input_reader))
    local input = json.decode(line)
    if input.type == "Initialize" then
        initialize = input
    end
end

function M.get_term_width()
    return initialize.term_width
end
function M.root_dir()
    return initialize.root_dir
end
function M.get_init_file()
    return initialize.init_file
end
function M.get_target_files()
    return initialize.target_files
end

---@param req lunest.bridge.Output
local function write_output(req)
    if not output_file then
        output_file = open_output_file()
    end
    output_file:writeln(json.encode(req))
end

---@param title string[]
function M.start_test(title)
    write_output({
        TestStarted = { title = title },
    })
end

---@param title string[]
---@param err lunest.bridge.TestError?
function M.finish_test(title, err)
    write_output({
        TestFinished = {
            title = title,
            error = err,
        },
    })
end

function M.finish()
    if input_file then
        input_file:close()
    end
    if output_file then
        output_file:close()
    end
end

return M
