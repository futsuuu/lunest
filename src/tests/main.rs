use std::path::Path;

use macros::lua_module_test;
use mlua::prelude::*;

#[cfg(test)]
use super::lua_eval;
use super::{group, test, TESTFILE};
use crate::{Group, MainState, Node, NodeID, State, Test};

fn set_state(lua: &Lua) -> LuaResult<()> {
    State::Main(MainState::new()).set(lua)?;
    Ok(())
}

#[lua_module_test(lua_eval)]
fn dont_execute_test(lua: &Lua) -> LuaResult<()> {
    set_state(lua)?;

    group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
        test(lua, TESTFILE, "test", |_lua| {
            unreachable!();
        })
    })?;

    Ok(())
}

mod load_test {
    use super::*;

    #[lua_module_test(lua_eval)]
    fn toplevel(lua: &Lua) -> LuaResult<()> {
        set_state(lua)?;

        group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
            test(lua, TESTFILE, "test", |_lua| {
                unreachable!();
            })
        })?;

        let state = State::get(lua)?;
        let state = state.borrow::<State>()?;
        let main_state = state.as_main().unwrap();

        let id = NodeID::from(vec![TESTFILE, "test"]);
        let node = main_state.get_node(&id);
        assert_eq!(&Node::from(Test::from(id)), node.unwrap());

        Ok(())
    }

    #[lua_module_test(lua_eval)]
    fn nested(lua: &Lua) -> LuaResult<()> {
        set_state(lua)?;

        group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
            group(lua, TESTFILE, "group 1", |lua| {
                group(lua, TESTFILE, "group 2", |lua| {
                    group(lua, TESTFILE, "group 3", |lua| {
                        group(lua, TESTFILE, "group 2", |lua| {
                            test(lua, TESTFILE, "test", |_lua| {
                                unreachable!();
                            })
                        })
                    })
                })
            })
        })?;

        let state = State::get(lua)?;
        let state = state.borrow::<State>()?;
        let main_state = state.as_main().unwrap();

        let id = NodeID::from(vec![
            TESTFILE, "group 1", "group 2", "group 3", "group 2", "test",
        ]);
        let node = main_state.get_node(&id).unwrap();
        assert_eq!(&Node::from(Test::from(id)), node);

        Ok(())
    }

    #[lua_module_test(lua_eval)]
    fn flat(lua: &Lua) -> LuaResult<()> {
        set_state(lua)?;

        group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
            group(lua, TESTFILE, "group 2", |_lua| Ok(()))?;
            group(lua, TESTFILE, "group 1", |lua| {
                test(lua, TESTFILE, "test", |_lua| {
                    unreachable!();
                })
            })?;
            group(lua, TESTFILE, "group 3", |_lua| Ok(()))?;
            Ok(())
        })?;

        let state = State::get(lua)?;
        let state = state.borrow::<State>()?;
        let main_state = state.as_main().unwrap();

        let id = NodeID::from(vec![TESTFILE, "group 1"]);
        let node = main_state.get_node(&id).unwrap();
        let mut group = Group::from(id);
        group.insert_node(
            Test::from(NodeID::from(vec![TESTFILE, "group 1", "test"])).into(),
        );
        assert_eq!(&Node::from(group), node);

        let id = NodeID::from(vec![TESTFILE, "group 2"]);
        let node = main_state.get_node(&id).unwrap();
        assert_eq!(&Node::from(Group::from(id)), node);

        let id = NodeID::from(vec![TESTFILE, "group 3"]);
        let node = main_state.get_node(&id).unwrap();
        assert_eq!(&Node::from(Group::from(id)), node);

        Ok(())
    }
}

#[lua_module_test(lua_eval)]
fn ignore_other_file(lua: &Lua) -> LuaResult<()> {
    set_state(lua)?;

    // load `TESTFILE`
    group(lua, TESTFILE, Path::new(TESTFILE), |lua| {
        // require('foo')
        group(lua, "foo.lua", Path::new("foo.lua"), |_lua| {
            unreachable!();
        })?;
        group(lua, TESTFILE, "group 2", |lua| {
            test(lua, TESTFILE, "test", |_lua| {
                unreachable!();
            })
        })?;
        group(lua, "other_test.lua", "group 3", |_lua| {
            unreachable!();
        })?;
        Ok(())
    })?;

    let state = State::get(lua)?;
    let state = state.borrow::<State>()?;
    let main_state = state.as_main().unwrap();

    let id = NodeID::from(vec![TESTFILE, "group 2", "test"]);
    let node = main_state.get_node(&id).unwrap();
    assert_eq!(&Node::from(Test::from(id)), node);

    Ok(())
}
