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
    return setmetatable({
        input = input,
        output = output,
        input_callbacks = {
            Initialize = {},
        },
    }, M)
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
                local field, input = next(json.decode(buf .. line))
                for _, callback in ipairs(self.input_callbacks[field] or {}) do
                    callback(input)
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
---@class lunest.Input
---@field Initialize? lunest.Input.Initialize
--- enum content
---@class lunest.Input.Initialize
---@field init_file string?
---@field root_dir string
---@field target_files { name: string, path: string }[]
---@field term_width integer

--- enum
---@class lunest.Output
---@field TestStarted? lunest.Output.TestStarted
---@field TestFinished? lunest.Output.TestFinished
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
