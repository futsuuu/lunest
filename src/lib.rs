mod cli;

use std::{
    env,
    io::{stdout, Write},
    ops::DerefMut,
    path::{Path, PathBuf},
    process::{Command, Stdio, exit},
};

use anyhow::{bail, Context, Result};
use globwalk::GlobWalkerBuilder;
use indexmap::IndexMap;
use mlua::prelude::*;

#[mlua::lua_module]
fn lunest(lua: &Lua) -> LuaResult<LuaTable> {
    lua.create_table_from([
        (
            "main",
            lua.create_function(|lua, args| {
                let cli = cli::Cli::new(args);
                match cli.args.command {
                    cli::Command::Test { name } => {
                        child_main(lua, name).into_lua_err()?;
                    }
                    cli::Command::Run { lua_cmd, pattern } => {
                        let mut lua_cmd = lua_cmd.clone();
                        lua_cmd.push(cli.main_file);
                        root_main(lua, &pattern, lua_cmd).into_lua_err()?;
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
            lua.create_function(|lua, (name, func)| {
                group(lua, name, func).into_lua_err()?;
                Ok(())
            })?,
        ),
    ])
}

fn root_main(lua: &Lua, patterns: &[String], lua_cmd: Vec<String>) -> Result<()> {
    let cwd = env::current_dir()?;
    let target_files = GlobWalkerBuilder::from_patterns(&cwd, patterns)
        .file_type(globwalk::FileType::FILE)
        .build()?
        .filter_map(Result::ok)
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<PathBuf>>();

    State::Root(RootState::new()).set(lua)?;
    for path in target_files {
        group(
            lua,
            path.strip_prefix(&cwd)
                .unwrap_or(&path)
                .display()
                .to_string(),
            lua.create_function(move |lua, _: ()| lua.load(path.as_path()).exec())?,
        )?;
    }

    fn spawn_children(
        node: &Node,
        name: &mut Vec<String>,
        lua_cmd: &[String],
    ) -> Result<()> {
        name.push(node.get_name().to_string());
        match node {
            Node::Group(g) => {
                for child in g.children.values() {
                    spawn_children(child, name, lua_cmd)?;
                }
            }
            Node::Test(_) => {
                let mut cmd = Command::new(&lua_cmd[0]);
                let name = &name[1..]; // Skip the "root" node
                cmd.args(&lua_cmd[1..])
                    .arg("test")
                    .args(name)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                print!("{} ····· ", name.join(" ┃ "));
                stdout().flush()?;

                let child = cmd.spawn().with_context(|| {
                    format!("Failed to spawn `{}`", command_to_string(&cmd))
                })?;
                let output = child.wait_with_output().with_context(|| {
                    format!("Failed to get the output of `{}`", command_to_string(&cmd))
                })?;

                if output.status.success() {
                    println!("OK");
                } else {
                    println!("ERR");
                    println!("{:─^30}", " stdout ");
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                    println!("{:─^30}", " stderr ");
                    println!("{}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }
        name.pop();
        Ok(())
    }
    let state = State::get(lua)?;
    let state = state.borrow::<State>()?;
    let state = state.as_root().unwrap();
    spawn_children(&state.tests, &mut Vec::new(), &lua_cmd)?;

    Ok(())
}

fn child_main(lua: &Lua, test: Vec<String>) -> Result<()> {
    let mut test = test.clone();
    let target_file = test.remove(0);
    State::Child(ChildState::new(test)).set(lua)?;
    lua.load(Path::new(&target_file)).exec()?;
    Ok(())
}

#[derive(Debug)]
enum State {
    Root(RootState),
    Child(ChildState),
}

impl LuaUserData for State {}

#[derive(Debug)]
struct RootState {
    group_stack: Vec<String>,
    tests: Node,
}

#[derive(Debug)]
struct ChildState {
    test: Vec<String>,
}

impl State {
    const REG_KEY: &'static str = concat!(env!("CARGO_PKG_NAME"), ".state");

    fn get(lua: &Lua) -> LuaResult<LuaAnyUserData> {
        lua.named_registry_value(Self::REG_KEY)
    }

    fn set(self, lua: &Lua) -> LuaResult<()> {
        lua.set_named_registry_value(Self::REG_KEY, self.into_lua(lua)?)
    }

    fn as_root(&self) -> Option<&RootState> {
        match self {
            Self::Root(r) => Some(r),
            _ => None,
        }
    }
}

impl RootState {
    fn new() -> Self {
        Self {
            group_stack: Vec::new(),
            tests: Node::Group(Group {
                name: String::from("root"),
                children: IndexMap::new(),
            }),
        }
    }

    fn add_node(&mut self, node: Node) -> Result<()> {
        let group = self.tests.get_child_mut(&self.group_stack)?;
        group
            .as_group_mut()?
            .children
            .insert(node.get_name().to_string(), node);
        Ok(())
    }
}

impl ChildState {
    fn new(test: Vec<String>) -> Self {
        Self { test }
    }
}

#[derive(Debug)]
enum Node {
    Group(Group),
    Test(Test),
}

impl Node {
    fn get_name(&self) -> &str {
        match self {
            Self::Group(g) => &g.name,
            Self::Test(t) => &t.name,
        }
    }

    fn as_group_mut(&mut self) -> Result<&mut Group> {
        match self {
            Self::Group(g) => Ok(g),
            _ => bail!("Cannot use {} as Group", self.get_name()),
        }
    }

    fn get_child_mut(&mut self, keys: &[String]) -> Result<&mut Self> {
        fn inner<'a>(
            node: &'a mut Node,
            keys: &'_ [String],
            count: usize,
        ) -> Result<&'a mut Node> {
            let Some(key) = keys.get(count) else {
                return Ok(node);
            };
            let node_name = node.get_name().to_string();
            let Some(child) = node.as_group_mut()?.children.get_mut(key) else {
                bail!("Failed to get {key} from {node_name}");
            };
            inner(child, keys, count + 1)
        }
        inner(self, keys, 0)
    }
}

#[derive(Debug)]
struct Group {
    name: String,
    children: IndexMap<String, Node>,
}

#[derive(Debug)]
struct Test {
    name: String,
}

fn test(lua: &Lua, name: String, func: LuaFunction) -> Result<()> {
    let state = State::get(lua)?;
    let mut state = state.borrow_mut::<State>()?;
    match state.deref_mut() {
        State::Root(ref mut root_state) => {
            let node = Node::Test(Test { name });
            root_state.add_node(node)?;
        }
        State::Child(child_state) => {
            if child_state.test.first() != Some(&name) {
                return Ok(());
            }
            match func.call(()) {
                Err(LuaError::RuntimeError(e)) => {
                    eprintln!("{e}");
                    exit(1);
                }
                r => r?,
            }
        }
    }

    Ok(())
}

fn group(lua: &Lua, name: String, func: LuaFunction) -> Result<()> {
    let state = State::get(lua)?;
    let mut state = state.borrow_mut::<State>()?;
    match state.deref_mut() {
        State::Root(ref mut root_state) => {
            let node = Node::Group(Group {
                name: name.clone(),
                children: IndexMap::new(),
            });
            root_state.add_node(node)?;
            root_state.group_stack.push(name.clone());
        }
        State::Child(ref mut child_state) => {
            if child_state.test.first() != Some(&name) {
                return Ok(());
            }
            child_state.test.remove(0);
        }
    }
    drop(state);

    func.call(())?;

    let state = State::get(lua)?;
    let mut state = state.borrow_mut::<State>()?;
    match state.deref_mut() {
        State::Root(ref mut root_state) => {
            root_state.group_stack.pop();
        }
        State::Child(ref mut child_state) => {
            child_state.test.insert(0, name);
        }
    }

    Ok(())
}

fn command_to_string(cmd: &Command) -> String {
    format!(
        "{} {}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args()
            .map(|s| s.to_string_lossy().escape_debug().to_string())
            .map(|s| if s.contains(' ') {
                format!("\"{s}\"")
            } else {
                s
            })
            .collect::<Vec<String>>()
            .join(" ")
    )
}
