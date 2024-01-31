use std::{
    io::{stdout, Write},
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use indexmap::IndexMap;

pub type Name = String;
pub type ID = Vec<Name>;

#[derive(Debug)]
pub enum Node {
    Group(Group),
    Test(Test),
}

impl Node {
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
        self.spawn_tests_inner(&mut ID::new(), lua_cmd)
    }

    fn spawn_tests_inner(&self, node_id: &mut ID, lua_cmd: &[String]) -> Result<()> {
        node_id.push(self.get_name().to_string());
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
    pub fn new(name: &str) -> Self {
        Group {
            name: Name::from(name),
            children: IndexMap::new(),
        }
    }

    pub fn insert_node(&mut self, node: Node) {
        self.children.insert(node.get_name().to_string(), node);
    }
}

#[derive(Debug)]
pub struct Test {
    name: Name,
}

impl Test {
    pub fn new(name: &str) -> Self {
        Test {
            name: Name::from(name),
        }
    }
}

fn spawn_test(node_id: &ID, lua_cmd: &[String]) -> Result<()> {
    let mut cmd = Command::new(&lua_cmd[0]);
    let node_id = &node_id[1..]; // Skip the root node
    cmd.args(&lua_cmd[1..])
        .arg("test")
        .args(node_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    print!("{} ····· ", node_id.join(" ┃ "));
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
