mod id;
mod name;

use std::{
    io::{stdout, Write},
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use shared::command_to_string;

pub use id::ID;
pub use name::Name;

#[derive(Debug)]
pub enum Node {
    Group(Group),
    Test(Test),
}

impl Node {
    const ROOT: &'static str = "__root__";

    pub fn root() -> Self {
        Group::new(Self::ROOT.to_string().into()).into()
    }

    pub fn get_name(&self) -> &Name {
        match self {
            Self::Group(g) => &g.name,
            Self::Test(t) => &t.name,
        }
    }

    pub fn as_group_mut(&mut self) -> Result<&mut Group> {
        match self {
            Self::Group(g) => Ok(g),
            _ => bail!("Cannot use {} as Group", self.get_name()),
        }
    }

    pub fn spawn_tests(&self, lua_cmd: &[String]) -> Result<()> {
        self.spawn_tests_inner(&mut ID::root(), lua_cmd)
    }

    fn spawn_tests_inner(&self, node_id: &mut ID, lua_cmd: &[String]) -> Result<()> {
        let name = self.get_name();
        if name.to_string().as_str() != Self::ROOT {
            node_id.push(name)?;
        }
        match self {
            Node::Group(g) => {
                for child in g.children.values() {
                    child.spawn_tests_inner(node_id, lua_cmd)?;
                }
            }
            Node::Test(_) => {
                spawn_test(node_id, lua_cmd)?;
            }
        }
        node_id.pop();
        Ok(())
    }
}

impl From<Group> for Node {
    fn from(value: Group) -> Self {
        Self::Group(value)
    }
}

impl From<Test> for Node {
    fn from(value: Test) -> Self {
        Self::Test(value)
    }
}

#[derive(Debug)]
pub struct Group {
    name: Name,
    pub children: IndexMap<Name, Node>,
}

impl Group {
    pub fn new(name: Name) -> Self {
        Group {
            name,
            children: IndexMap::new(),
        }
    }

    pub fn insert_node(&mut self, node: Node) {
        self.children.insert(node.get_name().clone(), node);
    }
}

#[derive(Debug)]
pub struct Test {
    name: Name,
}

impl Test {
    pub fn new(name: String) -> Self {
        Test {
            name: Name::from(name),
        }
    }
}

fn spawn_test(node_id: &ID, lua_cmd: &[String]) -> Result<()> {
    let mut cmd = Command::new(&lua_cmd[0]);
    cmd.args(&lua_cmd[1..])
        .arg("test")
        .args(node_id.clone())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    print!("{} ····· ", node_id);
    stdout().flush()?;

    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn `{}`", command_to_string(&cmd)))?;
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

    Ok(())
}
