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
        Run = {},
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
    local all_inputs_read = true
    while not self.input:is_closed() do
        local line = self.input:readln()
        if line then
            self:log("line: %s", line)
            all_inputs_read = false
            if line:sub(#line) == "\n" then
                ---@type lunest.Input
                local input = json.decode(buf .. line)
                self:log("calling callbacks of %q", input.t)
                for _, callback in ipairs(self.input_callbacks[input.t] or {}) do
                    callback(input.c)
                end
                buf = ""
            else
                buf = buf .. line
            end
        elseif not all_inputs_read and buf == "" then
            all_inputs_read = true
            self:write({ t = "AllInputsRead" })
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

---@param f fun(input: lunest.Input.Run)
function M:on_run(f)
    table.insert(self.input_callbacks.Run, f)
end

---@param f fun(script: string)
function M:on_execute(f)
    table.insert(self.input_callbacks.Execute, f)
end

---@param f fun()
function M:on_finish(f)
    table.insert(self.input_callbacks.Finish, f)
end

---@param s string
---@param ... any
function M:log(s, ...)
    if not self.output:is_closed() then
        return self:write({
            t = "Log",
            c = s:format(...),
        })
    end
end

---@param id string
---@param title string[]
function M:send_test_info(id, title)
    return self:write({
        t = "TestInfo",
        c = {
            id = id,
            title = title,
        },
    })
end

---@param title string[]
function M:notify_test_started(title)
    return self:write({
        t = "TestStarted",
        c = { title = title },
    })
end

---@param title string[]
---@param err lunest.TestError?
function M:notify_test_finished(title, err)
    return self:write({
        t = "TestFinished",
        c = {
            title = title,
            error = err,
        },
    })
end

--- enum
---@alias lunest.Input
---| { t: "Initialize", c: lunest.Input.Initialize }
---| { t: "Run", c: lunest.Input.Run }
---| { t: "Execute", c: string }
---| { t: "Finish", c: nil }
--- enum content
---@class lunest.Input.Initialize
---@field root_dir string
---@field target_files { name: string, path: string }[]
---@field term_width integer
--- enum content
---@class lunest.Input.Run
---@field test_id_filter string[]?
---@field test_mode lunest.TestMode
--- enum
---@alias lunest.TestMode
---| "Run"
---| "SendInfo"

--- enum
---@alias lunest.Output
---| { t: "TestInfo", c: lunest.Output.TestInfo }
---| { t: "TestStarted", c: lunest.Output.TestStarted }
---| { t: "TestFinished", c: lunest.Output.TestFinished }
---| { t: "AllInputsRead", c: nil }
---| { t: "Log", c: string }
--- enum content
---@class lunest.Output.TestInfo
---@field id string
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
