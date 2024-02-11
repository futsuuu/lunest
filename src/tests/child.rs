use macros::lua_module_test;
use mlua::prelude::*;

#[cfg(test)]
use super::lua_eval;
use super::{group, test};
use crate::{ChildState, NodeID, State};

fn set_state<'a, I>(lua: &Lua, id: I) -> LuaResult<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut id: Vec<String> = id.into_iter().map(|s| s.to_string()).collect();
    id.insert(0, String::from("test.lua"));
    let mut state = ChildState::new(NodeID::from(id));
    state.move_to_child(); // ignore "test.lua"
    State::Child(state).set(lua)?;
    Ok(())
}

mod execute_test {
    use super::*;

    #[lua_module_test(lua_eval)]
    fn toplevel(lua: &Lua) -> LuaResult<()> {
        set_state(lua, ["test"])?;

        test(lua, "test", |lua| lua.globals().set("executed", true))?;

        assert!(lua.globals().get::<_, bool>("executed")?);

        Ok(())
    }

    #[lua_module_test(lua_eval)]
    fn nested_group(lua: &Lua) -> LuaResult<()> {
        set_state(lua, ["group 1", "group 2", "group 3", "group 2", "test"])?;

        group(lua, "group 1", |lua| {
            group(lua, "group 2", |lua| {
                group(lua, "group 3", |lua| {
                    group(lua, "group 2", |lua| {
                        test(lua, "test", |lua| lua.globals().set("executed", true))
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
    set_state(lua, ["group 1", "test"])?;

    group(lua, "group 2", |_lua| {
        unreachable!();
    })?;
    group(lua, "group 1", |lua| {
        test(lua, "test", |lua| lua.globals().set("executed", true))
    })?;
    group(lua, "group 3", |_lua| {
        unreachable!();
    })?;

    assert!(lua.globals().get::<_, bool>("executed")?);

    Ok(())
}

