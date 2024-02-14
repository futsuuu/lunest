mod child;
mod main;

use std::path::Path;

use lunest_macros::lua_module_test;
use mlua::prelude::*;

use crate::NodeName;

const TESTFILE: &str = "test.lua";

#[lua_module_test(lua_eval)]
fn hello_world(lua: &Lua) -> LuaResult<()> {
    let lunest = super::lunest_lib(lua)?;
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
    let root = project_root::get_project_root().unwrap();
    // "lua_rt" package was already built by xtask
    let mut cmd = std::process::Command::new(root.join("target/debug/lua_rt"));
    cmd.arg(lua_code)
        .current_dir(root)
        .output()
        .expect("failed to execute process")
}

fn test<'lua, P, F>(lua: &'lua Lua, path: P, name: &'_ str, func: F) -> LuaResult<()>
where
    P: AsRef<Path>,
    F: Fn(&'lua Lua) -> LuaResult<()> + 'static,
{
    crate::test(
        lua,
        path.as_ref().to_path_buf(),
        name.to_string(),
        lua.create_function(move |lua, _: ()| func(lua))?,
    )
    .into_lua_err()
}

fn group<'lua, P, N, F>(lua: &'lua Lua, path: P, name: N, func: F) -> LuaResult<()>
where
    P: AsRef<Path>,
    N: Into<NodeName>,
    F: Fn(&'lua Lua) -> LuaResult<()> + 'static,
{
    crate::group(
        lua,
        path.as_ref().to_path_buf(),
        name.into(),
        lua.create_function(move |lua, _: ()| func(lua))?,
    )
    .into_lua_err()
}
