use clap::Parser as _;
use mlua::prelude::*;

#[mlua::lua_module]
fn lunest(lua: &Lua) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("cli", lua.create_function(cli)?)?;
    Ok(t)
}

fn cli(_lua: &Lua, args: Vec<String>) -> LuaResult<()> {
    let _args = Cli::parse_from(args);
    Ok(())
}

#[derive(clap::Parser)]
struct Cli {
    file: Vec<String>,
}
