mod convert;

use std::{collections::HashMap, env, path::PathBuf};

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
                root_main(lua, &args).into_lua_err()?;
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
                group(lua, None, name, func).into_lua_err()?;
                Ok(())
            })?,
        ),
    ])
}

fn root_main(lua: &Lua, args: &Cli) -> Result<()> {
    let cwd = env::current_dir()?;
    let target_files = GlobWalkerBuilder::from_patterns(&cwd, &args.patterns)
        .file_type(globwalk::FileType::FILE)
        .build()?
        .filter_map(Result::ok)
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<PathBuf>>();

    for path in target_files {
        group(
            lua,
            Some(path.clone()),
            path.strip_prefix(&cwd)
                .unwrap_or(&path)
                .display()
                .to_string(),
            lua.create_function(move |lua, _: ()| lua.load(path.as_path()).exec())?,
        )?;
    }

    println!("{:#?}", RootState::get(lua)?.tests.as_group().unwrap().children);

    Ok(())
}

#[derive(clap::Parser)]
struct Cli {
    #[arg(skip)]
    main_file: String,
    #[arg(default_values_t = [
        String::from(r"{test,spec}/**/*.lua"),
        String::from(r"*[-_\.]{test,spec}.lua"),
    ])]
    patterns: Vec<String>,
    #[arg(long, default_value = "lua")]
    lua_cmd: Vec<String>,
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
struct RootState {
    group_stack: Vec<String>,
    tests: Node,
}

impl RootState {
    const REG_KEY: &'static str = concat!(env!("CARGO_PKG_NAME"), ".state");

    fn new() -> Self {
        Self {
            group_stack: Vec::new(),
            tests: Node::Group(Group {
                name: String::from("root"),
                file: None,
                children: IndexMap::default(),
            }),
        }
    }

    fn get(lua: &Lua) -> LuaResult<Self> {
        Self::from_lua(lua.named_registry_value(Self::REG_KEY)?, lua)
    }

    fn set(self, lua: &Lua) -> LuaResult<()> {
        lua.set_named_registry_value(Self::REG_KEY, self.into_lua(lua)?)
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

    fn as_group(&self) -> Result<&Group> {
        match self {
            Self::Group(g) => Ok(g),
            _ => bail!("Cannot use {} as Group", self.get_name()),
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
    file: Option<PathBuf>,
    children: IndexMap<String, Node>,
}

#[derive(Debug)]
struct Test {
    name: String,
}

fn test(lua: &Lua, name: String, func: LuaFunction) -> Result<()> {
    let node = Node::Test(Test { name });
    let mut state = RootState::get(lua)?;
    state.add_node(node)?;
    state.set(lua)?;

    func.call(())?;
    Ok(())
}

fn group(
    lua: &Lua,
    file: Option<PathBuf>,
    name: String,
    func: LuaFunction,
) -> Result<()> {
    let node = Node::Group(Group {
        name: name.clone(),
        file,
        children: IndexMap::default(),
    });
    let mut state = RootState::get(lua)?;
    state.add_node(node)?;
    state.group_stack.push(name);
    state.set(lua)?;

    func.call(())?;

    let mut state = RootState::get(lua)?;
    state.group_stack.pop();
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
