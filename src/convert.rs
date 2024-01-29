use mlua::prelude::*;

use super::{ChildState, Group, IndexMap, Node, RootState, State, Test};

impl IntoLua<'_> for State {
    fn into_lua(self, lua: &'_ Lua) -> LuaResult<LuaValue<'_>> {
        let table = lua.create_table()?;
        match self {
            Self::Root(r) => {
                table.raw_push("root")?;
                table.raw_push(r)?;
            }
            Self::Child(c) => {
                table.raw_push("child")?;
                table.raw_push(c)?;
            }
        }
        table.into_lua(lua)
    }
}

impl FromLua<'_> for State {
    fn from_lua(value: LuaValue<'_>, lua: &'_ Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(value, lua)?;
        let inner = table.raw_get(2)?;
        let r = match table.raw_get::<_, String>(1)?.as_str() {
            "root" => Self::Root(RootState::from_lua(inner, lua)?),
            "child" => Self::Child(ChildState::from_lua(inner, lua)?),
            _ => unreachable!(),
        };
        Ok(r)
    }
}

impl IntoLua<'_> for RootState {
    fn into_lua(self, lua: &'_ Lua) -> LuaResult<LuaValue<'_>> {
        let table = lua.create_table()?;
        table.raw_set("group_stack", self.group_stack)?;
        table.raw_set("tests", self.tests)?;
        table.into_lua(lua)
    }
}

impl FromLua<'_> for RootState {
    fn from_lua(value: LuaValue<'_>, lua: &'_ Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(value, lua)?;
        Ok(Self {
            group_stack: table.raw_get("group_stack")?,
            tests: table.raw_get("tests")?,
        })
    }
}

impl IntoLua<'_> for ChildState {
    fn into_lua(self, lua: &'_ Lua) -> LuaResult<LuaValue<'_>> {
        let table = lua.create_table()?;
        table.raw_set("test", self.test)?;
        table.into_lua(lua)
    }
}

impl FromLua<'_> for ChildState {
    fn from_lua(value: LuaValue<'_>, lua: &'_ Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(value, lua)?;
        Ok(Self {
            test: table.raw_get("test")?,
        })
    }
}

impl IntoLua<'_> for Node {
    fn into_lua(self, lua: &'_ Lua) -> LuaResult<LuaValue<'_>> {
        let table = lua.create_table()?;
        match self {
            Self::Group(g) => {
                table.raw_push("group")?;
                table.raw_push(g)?;
            }
            Self::Test(t) => {
                table.raw_push("test")?;
                table.raw_push(t)?;
            }
            Self::Default => unreachable!(),
        }
        table.into_lua(lua)
    }
}

impl FromLua<'_> for Node {
    fn from_lua(value: LuaValue<'_>, lua: &'_ Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(value, lua)?;
        let inner = table.raw_get(2)?;
        let r = match table.raw_get::<_, String>(1)?.as_str() {
            "group" => Self::Group(Group::from_lua(inner, lua)?),
            "test" => Self::Test(Test::from_lua(inner, lua)?),
            _ => unreachable!(),
        };
        Ok(r)
    }
}

impl IntoLua<'_> for Group {
    fn into_lua(self, lua: &'_ Lua) -> LuaResult<LuaValue<'_>> {
        let table = lua.create_table()?;
        table.raw_set("name", self.name)?;
        table.raw_set("children", self.children)?;
        table.into_lua(lua)
    }
}

impl FromLua<'_> for Group {
    fn from_lua(value: LuaValue<'_>, lua: &'_ Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(value, lua)?;
        Ok(Self {
            name: table.raw_get("name")?,
            children: table.raw_get("children")?,
        })
    }
}

impl IntoLua<'_> for Test {
    fn into_lua(self, lua: &'_ Lua) -> LuaResult<LuaValue<'_>> {
        let table = lua.create_table()?;
        table.raw_set("name", self.name)?;
        table.into_lua(lua)
    }
}

impl FromLua<'_> for Test {
    fn from_lua(value: LuaValue<'_>, lua: &'_ Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(value, lua)?;
        Ok(Self {
            name: table.raw_get("name")?,
        })
    }
}

impl<'lua, K, V> FromLua<'lua> for IndexMap<K, V>
where
    K: FromLua<'lua> + Eq + std::hash::Hash,
    V: FromLua<'lua>,
{
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(value, lua)?;
        Ok(Self {
            vec: table.raw_get("vec")?,
            map: table.raw_get("map")?,
        })
    }
}

impl<'lua, K, V> IntoLua<'lua> for IndexMap<K, V>
where
    K: IntoLua<'lua> + Eq + std::hash::Hash,
    V: IntoLua<'lua>,
{
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        lua.create_table_from([
            ("vec", self.vec.into_lua(lua)?),
            ("map", self.map.into_lua(lua)?),
        ])?
        .into_lua(lua)
    }
}
