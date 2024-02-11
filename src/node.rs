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
    pub fn root() -> Self {
        Self::Group(Group::root())
    }

    pub fn id(&self) -> &ID {
        match self {
            Self::Group(g) => &g.id,
            Self::Test(t) => &t.id,
        }
    }

    pub fn as_group_mut(&mut self) -> Result<&mut Group> {
        match self {
            Self::Group(g) => Ok(g),
            _ => bail!("Cannot use {} as Group", self.id()),
        }
    }

    pub fn spawn_tests(&self, lua_cmd: &[String]) -> Result<()> {
        match self {
            Node::Test(t) => t.spawn(lua_cmd),
            Node::Group(g) => g.spawn_tests(lua_cmd),
        }
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
    id: ID,
    pub children: IndexMap<Name, Node>,
}

impl Group {
    pub fn root() -> Self {
        Self {
            id: ID::root(),
            children: IndexMap::new(),
        }
    }

    pub fn new(parent_id: ID, name: Name) -> Result<Self> {
        let mut id = parent_id;
        id.push(&name)?;
        Ok(Self {
            id,
            children: IndexMap::new(),
        })
    }

    pub fn insert_node(&mut self, node: Node) {
        self.children.insert(node.id().name().unwrap(), node);
    }

    fn spawn_tests(&self, lua_cmd: &[String]) -> Result<()> {
        for child in self.children.values() {
            child.spawn_tests(lua_cmd)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Test {
    id: ID,
}

impl Test {
    pub fn new(parent_id: ID, name: String) -> Result<Self> {
        let mut id = parent_id;
        id.push(&Name::from(name))?;
        Ok(Test { id })
    }

    fn spawn(&self, lua_cmd: &[String]) -> Result<()> {
        let mut cmd = Command::new(&lua_cmd[0]);
        cmd.args(&lua_cmd[1..])
            .arg("test")
            .args(self.id.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        print!("{} ····· ", self.id);
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
}
