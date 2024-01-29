mod convert;

use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};
use globwalk::GlobWalkerBuilder;
use mlua::prelude::*;

#[mlua::lua_module]
fn lunest(lua: &Lua) -> LuaResult<LuaTable> {
    lua.create_table_from([
        (
            "cli",
            lua.create_function(|lua, args| {
                let args = Cli::new(args);
                match args.command {
                    Subcommand::Test { name } => {
                        child_main(lua, name).into_lua_err()?;
                    }
                    Subcommand::Run { lua_cmd, pattern } => {
                        let mut lua_cmd = lua_cmd.clone();
                        lua_cmd.push(args.main_file);
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
                for key in &g.children.vec {
                    let value = &g.children.map[key];
                    spawn_children(value, name, lua_cmd)?;
                }
            }
            Node::Test(_) => {
                Command::new(&lua_cmd[0])
                    .args(&lua_cmd[1..])
                    .arg("test")
                    .args(&name[1..])
                    .status()?;
            }
            Node::Default => unreachable!(),
        }
        name.pop();
        Ok(())
    }
    let state = State::get(lua)?;
    let state = state.as_root().unwrap();
    spawn_children(&state.tests, &mut Vec::new(), &lua_cmd)?;

    Ok(())
}

fn child_main(lua: &Lua, test: Vec<String>) -> Result<()> {
    let mut test = test.clone();
    println!("{}", test.join(" > "));
    let target_file = test.remove(0);
    State::Child(ChildState::new(test)).set(lua)?;
    lua.load(Path::new(&target_file)).exec()?;
    Ok(())
}

#[derive(clap::Parser)]
struct Cli {
    #[arg(skip)]
    main_file: String,
    #[command(subcommand)]
    command: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    Run {
        #[arg(long, short, num_args = 1.., default_values_t = [
            String::from(r"{test,spec}/**/*.lua"),
            String::from(r"*[-_\.]{test,spec}.lua"),
        ])]
        pattern: Vec<String>,
        #[arg(long, default_value = "lua", num_args = 1.., allow_hyphen_values = true)]
        lua_cmd: Vec<String>,
    },
    #[command(hide = true)]
    Test {
        name: Vec<String>,
    },
}

impl Cli {
    fn new(args: Vec<String>) -> Self {
        use clap::Parser;
        let mut cli = Self::parse_from(&args);
        cli.main_file = args[0].clone();
        cli
    }
}

#[derive(Debug)]
enum State {
    Root(RootState),
    Child(ChildState),
}

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

    fn get(lua: &Lua) -> LuaResult<Self> {
        Self::from_lua(lua.named_registry_value(Self::REG_KEY)?, lua)
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
                children: IndexMap::default(),
            }),
        }
    }

    fn add_node(&mut self, node: Node) -> Result<()> {
        let group = self.tests.get_child_mut(&self.group_stack)?;
        group
            .as_group_mut()?
            .children
            .set(node.get_name().to_string(), node);
        Ok(())
    }
}

impl ChildState {
    fn new(test: Vec<String>) -> Self {
        Self { test }
    }
}

#[derive(Debug, Default)]
enum Node {
    #[default]
    Default,
    Group(Group),
    Test(Test),
}

impl Node {
    fn get_name(&self) -> &str {
        match self {
            Self::Group(g) => &g.name,
            Self::Test(t) => &t.name,
            Self::Default => unreachable!(),
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
    let mut state = State::get(lua)?;
    match state {
        State::Root(ref mut root_state) => {
            let node = Node::Test(Test { name });
            root_state.add_node(node)?;
            state.set(lua)?;
        }
        State::Child(child_state) => {
            if child_state.test.first() != Some(&name) {
                return Ok(());
            }
            func.call(())?;
        }
    }

    Ok(())
}

fn group(lua: &Lua, name: String, func: LuaFunction) -> Result<()> {
    let mut state = State::get(lua)?;
    match state {
        State::Root(ref mut root_state) => {
            let node = Node::Group(Group {
                name: name.clone(),
                children: IndexMap::default(),
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
    state.set(lua)?;

    func.call(())?;

    let mut state = State::get(lua)?;
    match state {
        State::Root(ref mut root_state) => {
            root_state.group_stack.pop();
        }
        State::Child(ref mut child_state) => {
            child_state.test.insert(0, name);
        }
    }
    state.set(lua)?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct IndexMap<K, V> {
    vec: Vec<K>,
    map: HashMap<K, V>,
}

impl<K, V> IndexMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    fn set(&mut self, key: K, value: V) {
        self.map.insert(key.clone(), value);
        self.vec.push(key);
    }
}
