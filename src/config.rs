use std::env::current_dir;

use mlua::prelude::*;

pub struct Config {
    pub lua_cmd: Vec<String>,
    pub pattern: Vec<String>,
}

impl Config {
    pub fn load(lua: &Lua, profile: &str) -> LuaResult<Self> {
        let file_path = current_dir()
            .unwrap()
            .join(".lunest")
            .join(profile)
            .with_extension("lua");
        lua.load(file_path).eval()
    }
}

impl FromLua<'_> for Config {
    fn from_lua(value: LuaValue<'_>, lua: &Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(value, lua)?;
        Ok(Self {
            lua_cmd: table.get("lua_cmd").unwrap_or(vec![
                String::from(r"{test,spec}/**/*.lua"),
                String::from(r"*[-_\.]{test,spec}.lua"),
            ]),
            pattern: table.get("pattern").unwrap_or(vec![String::from("lua")]),
        })
    }
}
