mod cli;
mod node;
mod state;
#[cfg(feature = "test")]
mod tests;

use std::{env, ops::DerefMut, path::PathBuf, process::exit};

use anyhow::Result;
use globwalk::GlobWalkerBuilder;
use mlua::prelude::*;

use node::{Group, Name as NodeName, Node, Test, ID as NodeID};
use state::{ChildState, MainState, State};

#[mlua::lua_module]
fn lunest(lua: &Lua) -> LuaResult<LuaTable> {
    lua.create_table_from([
        (
            "main",
            lua.create_function(|lua, args| {
                let cli = cli::Cli::new(args);
                match cli.args.command {
                    cli::Command::Run { lua_cmd, pattern } => {
                        let mut lua_cmd = lua_cmd.clone();
                        lua_cmd.push(cli.main_file);
                        main(lua, &pattern, lua_cmd).into_lua_err()?;
                    }
                    cli::Command::Test { id } => {
                        child_main(lua, NodeID::from(id)).into_lua_err()?;
                    }
                }
                Ok(())
            })?,
        ),
        (
            "test",
            lua.create_function(|lua, (name, func)| {
                test(lua, name, func).into_lua_err()?;
                Ok(())
            })?,
        ),
        (
            "group",
            lua.create_function(|lua, (name, func): (String, LuaFunction)| {
                group(lua, NodeName::from(name), func).into_lua_err()?;
                Ok(())
            })?,
        ),
    ])
}

fn main(lua: &Lua, patterns: &[String], lua_cmd: Vec<String>) -> Result<()> {
    let cwd = env::current_dir()?;
    let target_files = GlobWalkerBuilder::from_patterns(&cwd, patterns)
        .file_type(globwalk::FileType::FILE)
        .build()?
        .filter_map(Result::ok)
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<PathBuf>>();

    State::Main(MainState::new()).set(lua)?;
    for path in target_files {
        group(
            lua,
            path.strip_prefix(&cwd).unwrap_or(&path).to_path_buf(),
            lua.create_function(move |lua, _: ()| lua.load(path.as_path()).exec())?,
        )?;
    }

    let state = State::get(lua)?;
    let state = state.borrow::<State>()?;
    state.as_main().unwrap().root.spawn_tests(&lua_cmd)?;

    Ok(())
}

fn child_main(lua: &Lua, test: NodeID) -> Result<()> {
    let mut child_state = ChildState::new(test);
    let target_file = child_state.move_to_child().unwrap();
    let target_file = target_file.as_path().unwrap().to_path_buf();

    State::Child(child_state).set(lua)?;
    lua.load(target_file).exec()?;
    Ok(())
}

fn test(lua: &Lua, name: String, func: LuaFunction) -> Result<()> {
    let state = State::get(lua)?;
    let mut state = state.borrow_mut::<State>()?;
    match state.deref_mut() {
        State::Main(ref mut main_state) => {
            let parent_id = main_state.current_group.clone();
            main_state.insert_node(Test::new(parent_id, name)?)?;
        }
        State::Child(child_state) => {
            if !child_state.is_target(&name.into()) {
                return Ok(());
            }
            if let Err(e) = func.call::<_, ()>(()) {
                eprintln!("{e}");
                exit(1);
            }
        }
    }

    Ok(())
}

fn group<N: Into<NodeName>>(lua: &Lua, name: N, func: LuaFunction) -> Result<()> {
    let name = name.into();
    let state = State::get(lua)?;
    let mut state = state.borrow_mut::<State>()?;
    match state.deref_mut() {
        State::Main(ref mut main_state) => {
            let parent_id = main_state.current_group.clone();
            main_state.insert_node(Group::new(parent_id, name.clone())?)?;
            main_state.move_to_child(name)?;
        }
        State::Child(ref mut child_state) => {
            if !child_state.is_target(&name) {
                return Ok(());
            }
            child_state.move_to_child();
        }
    }
    drop(state);

    func.call(())?;

    let state = State::get(lua)?;
    let mut state = state.borrow_mut::<State>()?;
    match state.deref_mut() {
        State::Main(ref mut main_state) => main_state.move_to_parent(),
        State::Child(ref mut child_state) => child_state.move_to_parent(),
    }

    Ok(())
}
