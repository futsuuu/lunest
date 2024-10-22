if _G["vim"] then
    _G["vim"].opt.runtimepath:prepend(".")
else
    package.path = package.path
        .. (";lua/?.lua;lua/?/init.lua")
            :gsub("/", package.config:sub(1, 1))
            :gsub(";", package.config:sub(3, 3))
end
