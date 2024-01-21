#[mlua::lua_module]
fn lunest(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    Ok(t)
}
