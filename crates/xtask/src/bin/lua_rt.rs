use clap::Parser;

#[derive(Parser)]
struct Args {
    code: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let lua = unsafe { mlua::Lua::unsafe_new() };
    lua.load(&args.code).exec()?;
    Ok(())
}
