---@class lunest.File
---@field [1] file*
local M = {}
---@private
M.__index = M

---@param filename string
---@param mode openmode?
---@return self
function M.open(filename, mode)
    return setmetatable({ assert(io.open(filename, mode)) }, M)
end

function M:close()
    assert(self[1]:close())
end

---@return boolean
function M:is_closed()
    return io.type(self[1]) == "closed file"
end

---@param ... string | number
function M:write(...)
    assert(self[1]:write(...))
end

---@param ... string | number
function M:writeln(...)
    self:write(...)
    self:write("\n")
end

---@return integer
function M:size()
    local offset = self[1]:seek("cur")
    local n = self[1]:seek("end")
    self[1]:seek("set", offset)
    return n
end

---@return string?
function M:readln()
    if _G["jit"] or _VERSION ~= "Lua 5.1" then
        return self[1]:read("*L")
    end
    local line = self[1]:read("*l")
    if not line then
        return
    end
    self[1]:seek("cur", -1)
    if self[1]:read(1) == "\n" then
        line = line .. "\n"
    end
    return line
end

return M
