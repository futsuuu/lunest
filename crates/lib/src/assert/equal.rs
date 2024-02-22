use lunest_macros::lua_module_test;
#[cfg(feature = "test")]
use mlua::chunk;
use mlua::prelude::*;

#[cfg(test)]
use crate::tests::lua_eval;

pub fn equal(_lua: &Lua, v1: LuaValue, v2: LuaValue) -> LuaResult<bool> {
    if v1.equals(v2.clone())? {
        return Ok(true);
    }

    match (v1, v2) {
        (LuaValue::Table(t1), LuaValue::Table(t2)) => {
            for pair in t1.clone().pairs::<LuaValue, LuaValue>() {
                let (key, value) = pair?;
                if !equal(_lua, value, t2.get(key)?)? {
                    return Ok(false);
                }
            }
            for pair in t2.clone().pairs::<LuaValue, LuaValue>() {
                let (key, _) = pair?;
                if !t1.contains_key(key)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (LuaValue::Function(f1), LuaValue::Function(f2)) => {
            let f1 = f1.info();
            // All C functions return the same information
            if f1.what == "C" {
                return Ok(false);
            }
            let f2 = f2.info();
            Ok(f1.name == f2.name
                && f1.name_what == f2.name_what
                && f1.source == f2.source
                && f1.what == f2.what
                && f1.short_src == f2.short_src
                && f1.line_defined == f2.line_defined)
        }
        _ => Ok(false),
    }
}

#[cfg(feature = "test")]
fn equal_lua(lua: &Lua, (v1, v2): (LuaValue, LuaValue)) -> LuaResult<bool> {
    equal(lua, v1, v2)
}

#[lua_module_test(lua_eval)]
fn c_function(lua: &Lua) -> LuaResult<()> {
    let eq = lua.create_function(equal_lua)?;
    lua.load(chunk! {
        assert($eq(print, print))
        assert(not $eq(assert, print))
    })
    .exec()
}

#[lua_module_test(lua_eval)]
fn lua_function(lua: &Lua) -> LuaResult<()> {
    let eq = lua.create_function(equal_lua)?;

    let lua_func: LuaFunction = lua
        .load(chunk! {
            return function() end
        })
        .eval()?;

    lua.load(chunk! {
        local function get_luafunc()
            return function() end
        end
        local function other_func() end
        assert(get_luafunc() ~= get_luafunc())
        assert($eq(get_luafunc(), get_luafunc()))
        assert(not $eq($lua_func, get_luafunc()))
    })
    .exec()
}

#[lua_module_test(lua_eval)]
fn c_and_lua_function(lua: &Lua) -> LuaResult<()> {
    let eq = lua.create_function(equal_lua)?;
    lua.load(chunk! {
        local function print() end
        assert(not $eq(prnt, _G.print))
    })
    .exec()
}
