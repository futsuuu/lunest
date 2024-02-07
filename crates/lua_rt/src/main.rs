use clap::Parser;
use mlua::prelude::*;

#[derive(Parser)]
struct Args {
    code: String,
}

fn main() -> LuaResult<()> {
    let args = Args::parse();
    let lua = unsafe { Lua::unsafe_new() };
    lua.load(&args.code).exec()?;
    Ok(())
}
