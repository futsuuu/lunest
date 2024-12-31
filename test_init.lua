package.path = package.path
    .. ";"
    .. table.concat({
        "./lua/?.lua",
        "./lua/?/init.lua",
        "./lib/json.lua/?.lua",
    }, ";")

