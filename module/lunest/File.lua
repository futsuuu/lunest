---@class lunest.File
---@field [1] file*
local M = {}

local test = require("lunest.wrapper")
local assertion = test.assertion

---@private
M.__index = M

---@param file file*
---@return self
function M.from_raw(file)
    return setmetatable({ file }, M)
end

---@param filename string
---@param mode openmode?
---@return self
function M.open(filename, mode)
    return M.from_raw(assert(io.open(filename, mode)))
end

---@return self
function M.tmp()
    return M.from_raw(io.tmpfile())
end

function M:close()
    assert(self[1]:close())
end

---@return boolean
function M:is_closed()
    return io.type(self[1]) == "closed file"
end

test.test("close", function()
    local file = M.tmp()
    assert(not file:is_closed())
    file:close()
    assert(file:is_closed())
end)

---@param ... string | number
function M:write(...)
    assert(self[1]:write(...))
end

---@param ... string | number
function M:writeln(...)
    self:write(...)
    self:write("\n")
end

---@private
---@param whence seekwhence?
---@param offset integer?
---@return integer
function M:_seek(whence, offset)
    local offset_, err = self[1]:seek(whence, offset)
    assert(offset_ and not err, err)
    return offset_
end

---@param offset integer?
---@return integer
function M:seek(offset)
    return self:_seek("set", offset)
end

---@param offset integer?
---@return integer
function M:seek_end(offset)
    return self:_seek("end", -offset)
end

---@param offset integer?
---@return integer
function M:seek_rel(offset)
    return self:_seek("cur", offset)
end

if _G["jit"] or _VERSION ~= "Lua 5.1" then
    function M:readln()
        return self[1]:read("*L")
    end
else
    ---@return string?
    function M:readln()
        local line = self[1]:read("*l")
        if not line then
            return
        end
        self:seek_rel(-1)
        if self[1]:read(1) == "\n" then
            line = line .. "\n"
        end
        return line
    end
end

test.test("readln", function()
    local file = M.tmp()
    file:write("hello\nworld")
    file:seek()
    assertion.eq("hello\n", file:readln())
    assertion.eq("world", file:readln())
end)

return M
