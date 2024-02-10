mod child;

use macros::lua_module_test;
use mlua::prelude::*;

use crate::NodeName;

#[lua_module_test(lua_eval)]
fn hello_world(lua: &Lua) -> LuaResult<()> {
    let lunest = super::lunest(lua)?;
    lua.load(mlua::chunk! {
        local lunest = $lunest
        assert(type(lunest) == "table")
        assert(type(lunest.test) == "function")
        assert(type(lunest.group) == "function")
    })
    .exec()
}

#[cfg(test)]
fn lua_eval(lua_code: &str) -> std::process::Output {
    std::process::Command::new(env!("CARGO"))
        .args([
            "run",
            "--quiet", // "lua_rt" package was already built by "xtask"
            "--package",
            "lua_rt",
            "--features",
            macros::lua_feature!(),
            "--",
            lua_code,
        ])
        .output()
        .expect("failed to execute process")
}

fn test<'lua, F>(lua: &'lua Lua, name: &'_ str, func: F) -> LuaResult<()>
where
    F: Fn(&'lua Lua) -> LuaResult<()> + 'static,
{
    crate::test(
        lua,
        name.to_string(),
        lua.create_function(move |lua, _: ()| func(lua))?,
    )
    .into_lua_err()
}

fn group<'lua, F>(lua: &'lua Lua, name: &'_ str, func: F) -> LuaResult<()>
where
    F: Fn(&'lua Lua) -> LuaResult<()> + 'static,
{
    crate::group(
        lua,
        NodeName::from(name.to_string()),
        lua.create_function(move |lua, _: ()| func(lua))?,
    )
    .into_lua_err()
}
