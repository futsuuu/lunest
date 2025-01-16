---@class lunest.Process
---@field private input lunest.File
---@field private output lunest.File
---@field private input_callbacks table<string, function[]>
local M = {}
---@private
M.__index = M

local json = require("json")

local File = require("lunest.File")

---@param input lunest.File
---@param output lunest.File
---@return self
function M.new(input, output)
    local self = setmetatable({}, M)
    self.input = input
    self.output = output
    self.input_callbacks = {
        Initialize = {},
        TestFile = {},
        Execute = {},
        Finish = {},
    }
    self:on_finish(function()
        self:close()
    end)
    return self
end

---@param input string
---@param output string
---@return self
function M.open(input, output)
    return M.new(File.open(input, "r"), File.open(output, "a"))
end

function M:close()
    self.input:close()
    self.output:close()
end

function M:loop()
    local buf = ""
    while not self.input:is_closed() do
        local line = self.input:readln()
        if line then
            if line:sub(#line) == "\n" then
                ---@type lunest.Input
                local input = json.decode(buf .. line)
                for _, callback in ipairs(self.input_callbacks[input.t] or {}) do
                    callback(input.c)
                end
                buf = ""
            else
                buf = buf .. line
            end
        end
    end
end

---@param output lunest.Output
function M:write(output)
    return self.output:writeln(json.encode(output))
end

---@param f fun(input: lunest.Input.Initialize)
function M:on_initialize(f)
    table.insert(self.input_callbacks.Initialize, f)
end

---@param f fun(file: lunest.Input.TestFile)
function M:on_test_file(f)
    table.insert(self.input_callbacks.TestFile, f)
end

---@param f fun(script: string)
function M:on_execute(f)
    table.insert(self.input_callbacks.Execute, f)
end

---@param f fun()
function M:on_finish(f)
    table.insert(self.input_callbacks.Finish, f)
end

---@param title string[]
function M:notify_test_found(title)
    return self:write({
        TestFound = {
            title = title,
        },
    })
end

---@param title string[]
function M:notify_test_started(title)
    return self:write({
        TestStarted = {
            title = title,
        },
    })
end

---@param title string[]
---@param err lunest.TestError?
function M:notify_test_finished(title, err)
    return self:write({
        TestFinished = {
            title = title,
            error = err,
        },
    })
end

--- enum
---@alias lunest.Input
---| { t: "Initialize", c: lunest.Input.Initialize }
---| { t: "TestFile", c: lunest.Input.TestFile }
---| { t: "Execute", c: string }
---| { t: "Finish", c: nil }
--- enum content
---@class lunest.Input.Initialize
---@field mode "Run" | "List"
---@field root_dir string
---@field term_width integer
--- enum content
---@class lunest.Input.TestFile
---@field name string
---@field path string

--- enum
---@class lunest.Output
---@field TestFound? lunest.Output.TestFound
---@field TestStarted? lunest.Output.TestStarted
---@field TestFinished? lunest.Output.TestFinished
--- enum content
---@class lunest.Output.TestFound
---@field title string[]
--- enum content
---@class lunest.Output.TestStarted
---@field title string[]
--- enum content
---@class lunest.Output.TestFinished
---@field title string[]
---@field error lunest.TestError?

--- struct
---@class lunest.TestError
---@field message string
---@field traceback string
---@field info lunest.TestErrorInfo?

--- enum
---@class lunest.TestErrorInfo
---@field Diff? { left: string, right: string }

return M
