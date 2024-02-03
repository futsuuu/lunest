use anyhow::{bail, Result};
use mlua::prelude::*;

use crate::{Node, NodeID, NodeName};

#[derive(Debug)]
pub enum State {
    Main(MainState),
    Child(ChildState),
}

impl LuaUserData for State {}

impl State {
    const REG_KEY: &'static str = concat!(env!("CARGO_PKG_NAME"), ".state");

    pub fn get(lua: &Lua) -> LuaResult<LuaAnyUserData> {
        lua.named_registry_value(Self::REG_KEY)
    }

    pub fn set(self, lua: &Lua) -> LuaResult<()> {
        lua.set_named_registry_value(Self::REG_KEY, self.into_lua(lua)?)
    }

    pub fn as_main(&self) -> Option<&MainState> {
        match self {
            Self::Main(r) => Some(r),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct MainState {
    pub root: Node,
    current_group: NodeID,
}

impl MainState {
    pub fn new() -> Self {
        Self {
            root: Node::root(),
            current_group: NodeID::root(),
        }
    }

    pub fn insert_node<N: Into<Node>>(&mut self, node: N) -> Result<()> {
        self.get_node(&self.current_group.clone())?
            .as_group_mut()?
            .insert_node(node.into());
        Ok(())
    }

    pub fn move_to_child(&mut self, name: NodeName) -> Result<()> {
        self.current_group.push(&name)
    }

    pub fn move_to_parent(&mut self) {
        self.current_group.pop();
    }

    fn get_node(&mut self, id: &NodeID) -> Result<&mut Node> {
        fn inner<'a>(
            node: &'a mut Node,
            node_id: &'_ NodeID,
            depth: usize,
        ) -> Result<&'a mut Node> {
            let Some(child_name) = node_id.get(depth) else {
                // node_id.len() ≦ depth
                return Ok(node);
            };
            let node_name = node.get_name().to_string();
            let Some(child) = node.as_group_mut()?.children.get_mut(&child_name) else {
                bail!("Failed to get {child_name} from {node_name}");
            };
            inner(child, node_id, depth + 1)
        }
        inner(&mut self.root, id, 0)
    }
}

#[derive(Debug)]
pub struct ChildState {
    target: NodeID,
    depth: usize,
}

impl ChildState {
    pub fn new(test: NodeID) -> Self {
        Self {
            target: test,
            depth: 0,
        }
    }

    pub fn is_target(&self, name: &NodeName) -> bool {
        &self.target.get(self.depth).unwrap() == name
    }

    pub fn move_to_child(&mut self) -> Option<NodeName> {
        let r = self.target.get(self.depth);
        self.depth += 1;
        r
    }

    pub fn move_to_parent(&mut self) {
        self.depth -= 1;
    }
}
