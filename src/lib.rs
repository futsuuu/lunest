mod convert;

use std::{collections::HashMap, env, path::PathBuf};

use globwalk::GlobWalkerBuilder;
use mlua::prelude::*;

#[mlua::lua_module]
fn lunest(lua: &Lua) -> LuaResult<LuaTable> {
    lua.create_table_from([
        (
            "cli",
            lua.create_function(|lua, args| {
                Cli::new(args).start_main(lua)?;
                Ok(())
            })?,
        ),
        (
            "test",
            lua.create_function(|lua, (name, func)| {
                test(lua, name, func)?;
                Ok(())
            })?,
        ),
        (
            "group",
            lua.create_function(|lua, (name, func)| {
                group(lua, name, func)?;
                Ok(())
            })?,
        ),
    ])
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
    #[arg(long, short, default_value = "lua")]
    lua_cmd: Vec<String>,
}

impl Cli {
    fn new(args: Vec<String>) -> Self {
        use clap::Parser;
        let mut cli = Self::parse_from(&args);
        cli.main_file = args[0].clone();
        cli
    }

    fn start_main(&self, lua: &Lua) -> LuaResult<()> {
        let cwd = env::current_dir().into_lua_err()?;
        let target_files: Vec<PathBuf> =
            GlobWalkerBuilder::from_patterns(&cwd, &self.patterns)
                .file_type(globwalk::FileType::FILE)
                .build()
                .into_lua_err()?
                .filter_map(Result::ok)
                .map(|e| e.path().to_path_buf())
                .collect();

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

        println!("{:?}", State::get(lua)?.tests);

        Ok(())
    }
}

#[derive(Debug)]
struct State {
    group_stack: Vec<String>,
    tests: Group,
}

impl State {
    const REG_KEY: &'static str = concat!(env!("CARGO_PKG_NAME"), ".state");

    fn new() -> Self {
        Self {
            group_stack: Vec::new(),
            tests: Group {
                name: String::from("root"),
                children: IndexMap::default(),
            },
        }
    }

    fn get(lua: &Lua) -> LuaResult<Self> {
        Self::from_lua(lua.named_registry_value(Self::REG_KEY)?, lua)
    }

    fn set(self, lua: &Lua) -> LuaResult<()> {
        lua.set_named_registry_value(Self::REG_KEY, self.into_lua(lua)?)
    }

    fn add_node(&mut self, node: Node) {
        let group = self.tests.get_child_group(&self.group_stack);
        group.children.set(node.get_name().to_string(), node);
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
}

#[derive(Debug)]
struct Group {
    name: String,
    children: IndexMap<String, Node>,
}

impl Group {
    fn get_child_group(&mut self, keys: &[String]) -> &mut Group {
        fn inner<'a>(
            group: &'a mut Group,
            keys: &'_ [String],
            count: usize,
        ) -> &'a mut Group {
            let Some(key) = keys.get(count) else {
                return group;
            };
            if let Some(Node::Group(g)) = group.children.get_mut(key) {
                return inner(g, keys, count + 1);
            }
            unreachable!();
        }
        inner(self, keys, 0)
    }
}

#[derive(Debug)]
struct Test {
    name: String,
}

fn test(lua: &Lua, name: String, func: LuaFunction) -> LuaResult<()> {
    let node = Node::Test(Test { name });
    let mut state = State::get(lua)?;
    state.add_node(node);
    state.set(lua)?;

    func.call(())
}

fn group(lua: &Lua, name: String, func: LuaFunction) -> LuaResult<()> {
    let node = Node::Group(Group {
        name: name.clone(),
        children: IndexMap::default(),
    });
    let mut state = State::get(lua)?;
    state.add_node(node);
    state.group_stack.push(name);
    state.set(lua)?;

    func.call(())?;

    let mut state = State::get(lua)?;
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
