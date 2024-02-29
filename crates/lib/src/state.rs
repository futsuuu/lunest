use std::path::PathBuf;

use anyhow::{Context, Result};
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

macro_rules! get_state {
    ($lua:expr, mut $state:ident) => {
        let $state = crate::state::State::get($lua)?;
        let mut $state = $state.borrow_mut::<crate::state::State>()?;
    };
    ($lua:expr, $state:ident) => {
        let $state = crate::state::State::get($lua)?;
        let $state = $state.borrow::<crate::state::State>()?;
    };
}
pub(crate) use get_state;

#[derive(Debug)]
pub struct MainState {
    pub root: Node,
    pub current_group: NodeID,
}

impl MainState {
    pub fn new() -> Self {
        Self {
            root: Node::root(),
            current_group: NodeID::root(),
        }
    }

    pub fn insert_node<N: Into<Node>>(&mut self, node: N) -> Result<()> {
        let node = node.into();
        let current_group = &self.current_group.to_string();
        self.get_node_mut(&self.current_group.clone())
            .and_then(|node| node.as_group_mut())
            .with_context(|| format!("{current_group} is not a Group node"))?
            .insert_node(node);
        Ok(())
    }

    pub fn move_to_child(&mut self, name: NodeName) -> Result<()> {
        self.current_group.push(&name)
    }

    pub fn move_to_parent(&mut self) {
        self.current_group.pop();
    }

    pub fn is_target(&self, path: &PathBuf) -> bool {
        let Some(current) = self.current_group.get(0) else {
            return true;
        };
        let current = current.as_path().unwrap();
        if let (Ok(current), Ok(path)) = (current.canonicalize(), path.canonicalize()) {
            current == path
        } else {
            current == path
        }
    }

    pub fn get_node_mut(&mut self, id: &NodeID) -> Option<&mut Node> {
        fn inner<'a>(
            node: &'a mut Node,
            node_id: &'_ NodeID,
            depth: usize,
        ) -> Option<&'a mut Node> {
            let Some(child_name) = node_id.get(depth) else {
                // node_id.len() ≦ depth
                return Some(node);
            };
            let Some(group) = node.as_group_mut() else {
                return None;
            };
            let Some(child) = group.children.get_mut(&child_name) else {
                return None;
            };
            inner(child, node_id, depth + 1)
        }
        inner(&mut self.root, id, 0)
    }

    #[cfg(feature = "test")]
    pub fn get_node(&self, id: &NodeID) -> Option<&Node> {
        fn inner<'a>(
            node: &'a Node,
            node_id: &'_ NodeID,
            depth: usize,
        ) -> Option<&'a Node> {
            let Some(child_name) = node_id.get(depth) else {
                return Some(node);
            };
            let Some(group) = node.as_group() else {
                return None;
            };
            let Some(child) = group.children.get(&child_name) else {
                return None;
            };
            inner(child, node_id, depth + 1)
        }
        inner(&self.root, id, 0)
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

    pub fn is_target(&self, path: &PathBuf, name: &NodeName) -> bool {
        self.target.get(0).unwrap().as_path() == Some(path)
            && &self.target.get(self.depth).unwrap() == name
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
