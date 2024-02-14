use std::env;

use mlua::prelude::*;

pub struct Cli {
    pub args: Args,
    pub lua_cmd: Vec<String>,
}

#[derive(clap::Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Run {
        #[arg(default_value = "default")]
        profile: String,
    },
    #[command(hide = true)]
    Test {
        #[arg(long)]
        profile: String,
        id: Vec<String>,
    },
}

impl Cli {
    pub fn new(lua: &Lua) -> LuaResult<Self> {
        use clap::Parser;

        // ['lua', 'file.lua', 'arg1', 'arg2']
        let args = env::args().collect::<Vec<String>>();
        // ['arg1', 'arg2']
        let lua_args = lua.globals().get::<_, Vec<String>>("arg")?;
        // ['lua', 'file.lua']
        let lua_cmd = args[..args.len() - lua_args.len()].to_vec();
        // ['file.lua', 'arg1', 'arg2']
        let args = {
            let mut args = lua_args.clone();
            args.insert(0, lua_cmd.last().unwrap().clone());
            args
        };
        Ok(Self {
            args: Args::parse_from(args),
            lua_cmd,
        })
    }
}
