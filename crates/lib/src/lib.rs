mod assert;
mod node;
mod state;
#[cfg(feature = "test")]
mod tests;

use std::{env, ops::DerefMut, path::PathBuf, process::exit};

use anyhow::Result;
use globwalk::GlobWalkerBuilder;
use lunest_shared::{
    cli::{self, Parser as _},
    config::{Config, Profile},
};
use mlua::prelude::*;

use node::{Group, Name as NodeName, Node, Test, ID as NodeID};
use state::{ChildState, MainState, State};

#[mlua::lua_module]
fn lunest_lib(lua: &Lua) -> LuaResult<LuaTable> {
    lua.create_table_from([
        (
            "main",
            lua.create_function(|lua, _: ()| {
                main(lua).into_lua_err()?;
                Ok(())
            })?,
        ),
        (
            "test",
            lua.create_function(
                |lua, (pos, name, func): (CalledPos, String, LuaFunction)| {
                    test(lua, pos.path, name, func).into_lua_err()?;
                    Ok(())
                },
            )?,
        ),
        (
            "group",
            lua.create_function(
                |lua, (pos, name, func): (CalledPos, String, LuaFunction)| {
                    group(lua, pos.path, NodeName::from(name), func).into_lua_err()?;
                    Ok(())
                },
            )?,
        ),
        ("assert", lua.create_function(assert::assert)?),
        ("assert_eq", lua.create_function(assert::assert_eq)?),
        ("assert_ne", lua.create_function(assert::assert_ne)?),
    ])
}

fn main(lua: &Lua) -> Result<()> {
    let args = cli::Args::parse_from(lua_args(lua)?);
    let profile = Config::load()?.get_profile(&args.profile)?;
    match args.command {
        cli::Command::Run => (),
        cli::Command::Test { id } => {
            return child_main(lua, profile, NodeID::from(id));
        }
    }
    let cwd = env::current_dir()?;
    let target_files = GlobWalkerBuilder::from_patterns(&cwd, profile.get_files())
        .file_type(globwalk::FileType::FILE)
        .build()?
        .filter_map(Result::ok)
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<PathBuf>>();

    if let Some(setup) = profile.get_setup() {
        lua.load(setup).exec()?;
    }
    State::Main(MainState::new()).set(lua)?;
    for path in target_files {
        group(
            lua,
            path.clone(),
            path.strip_prefix(&cwd).unwrap_or(&path).to_path_buf(),
            lua.create_function(move |lua, _: ()| lua.load(path.as_path()).exec())?,
        )?;
    }

    let mut lua_cmd = profile.get_lua().clone();
    let main_file = lua.globals().get::<_, LuaTable>("arg")?.get(0)?;
    lua_cmd.push(main_file);

    let state = State::get(lua)?;
    let state = state.borrow::<State>()?;
    state
        .as_main()
        .unwrap()
        .root
        .spawn_tests(&lua_cmd, profile.get_name())?;

    Ok(())
}

fn child_main(lua: &Lua, profile: Profile, test: NodeID) -> Result<()> {
    let mut child_state = ChildState::new(test);
    let target_file = child_state.move_to_child().unwrap();
    let target_file = target_file.as_path().unwrap().to_path_buf();

    if let Some(setup) = profile.get_setup() {
        lua.load(setup).exec()?;
    }
    State::Child(child_state).set(lua)?;
    lua.load(target_file).exec()?;
    Ok(())
}

fn test(lua: &Lua, path: PathBuf, name: String, func: LuaFunction) -> Result<()> {
    let state = State::get(lua)?;
    let mut state = state.borrow_mut::<State>()?;
    match state.deref_mut() {
        State::Main(ref mut main_state) => {
            if !main_state.is_target(&path) {
                return Ok(());
            }
            let parent_id = main_state.current_group.clone();
            main_state.insert_node(Test::new(parent_id, name)?)?;
        }
        State::Child(child_state) => {
            if !child_state.is_target(&path, &name.into()) {
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

fn group<N>(lua: &Lua, path: PathBuf, name: N, func: LuaFunction) -> Result<()>
where
    N: Into<NodeName>,
{
    let name = name.into();
    let state = State::get(lua)?;
    let mut state = state.borrow_mut::<State>()?;
    match state.deref_mut() {
        State::Main(ref mut main_state) => {
            if !main_state.is_target(&path) {
                return Ok(());
            }
            let parent_id = main_state.current_group.clone();
            main_state.insert_node(Group::new(parent_id, name.clone())?)?;
            main_state.move_to_child(name)?;
        }
        State::Child(ref mut child_state) => {
            if !child_state.is_target(&path, &name) {
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

fn lua_args(lua: &Lua) -> LuaResult<Vec<String>> {
    let args = lua.globals().get::<_, LuaValue>("arg")?;
    let mut vec = Vec::<String>::from_lua(args.clone(), lua)?;
    vec.insert(0, LuaTable::from_lua(args, lua)?.get::<_, String>(0)?);
    Ok(vec)
}

struct CalledPos {
    path: PathBuf,
    line: usize,
}

impl FromLua<'_> for CalledPos {
    fn from_lua(value: LuaValue<'_>, lua: &Lua) -> LuaResult<Self> {
        let value = LuaTable::from_lua(value, lua)?;
        Ok(Self {
            path: PathBuf::from(value.get::<_, String>("path")?),
            line: value.get::<_, LuaInteger>("line")? as usize,
        })
    }
}
