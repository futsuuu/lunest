use std::path::Path;

use lunest_macros::lua_module_test;
use mlua::prelude::*;

#[cfg(test)]
use super::lua_eval;
use super::{group, test, TESTFILE};
use crate::{ChildState, NodeID, State};

fn set_state<'a, I>(lua: &Lua, id: I) -> LuaResult<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let id: Vec<String> = id.into_iter().map(|s| s.to_string()).collect();
    let state = ChildState::new(NodeID::from(id));
    State::Child(state).set(lua)?;
    Ok(())
}

mod execute_test {
    use super::*;

    #[lua_module_test(lua_eval)]
    fn toplevel(lua: &Lua) -> LuaResult<()> {
        set_state(lua, [TESTFILE, "test"])?;

        group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
            test(lua, TESTFILE, "test", |lua| {
                lua.globals().set("executed", true)
            })
        })?;

        assert!(lua.globals().get::<_, bool>("executed")?);

        Ok(())
    }

    #[lua_module_test(lua_eval)]
    fn nested_group(lua: &Lua) -> LuaResult<()> {
        set_state(
            lua,
            [TESTFILE, "group 1", "group 2", "group 3", "group 2", "test"],
        )?;

        group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
            group(lua, TESTFILE, "group 1", |lua| {
                group(lua, TESTFILE, "group 2", |lua| {
                    group(lua, TESTFILE, "group 3", |lua| {
                        group(lua, TESTFILE, "group 2", |lua| {
                            test(lua, TESTFILE, "test", |lua| {
                                lua.globals().set("executed", true)
                            })
                        })
                    })
                })
            })
        })?;

        assert!(lua.globals().get::<_, bool>("executed")?);

        Ok(())
    }
}

#[lua_module_test(lua_eval)]
fn ignore_other_group(lua: &Lua) -> LuaResult<()> {
    set_state(lua, [TESTFILE, "group 1", "test"])?;

    group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
        group(lua, TESTFILE, "group 2", |_lua| {
            unreachable!();
        })?;
        group(lua, TESTFILE, "group 1", |lua| {
            test(lua, TESTFILE, "test", |lua| {
                lua.globals().set("executed", true)
            })
        })?;
        group(lua, TESTFILE, "group 3", |_lua| {
            unreachable!();
        })?;
        Ok(())
    })?;

    assert!(lua.globals().get::<_, bool>("executed")?);

    Ok(())
}

#[lua_module_test(lua_eval)]
fn ignore_other_file(lua: &Lua) -> LuaResult<()> {
    set_state(lua, [TESTFILE, "group 1", "test"])?;

    group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
        group(lua, "foo.lua", "group 1", |_lua| {
            unreachable!();
        })?;
        group(lua, TESTFILE, "group 1", |lua| {
            test(lua, TESTFILE, "test", |lua| {
                lua.globals().set("executed", true)
            })
        })?;
        group(lua, "other_test.lua", "group 1", |_lua| {
            unreachable!();
        })?;
        Ok(())
    })?;

    assert!(lua.globals().get::<_, bool>("executed")?);

    Ok(())
}
