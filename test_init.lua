package.path = package.path
    .. ";"
    .. table.concat({
        "./module/?.lua",
        "./module/?/init.lua",
        "./3rd/json.lua/?.lua",
    }, ";")

