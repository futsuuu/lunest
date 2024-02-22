use mlua::prelude::*;

pub fn equal(_lua: &Lua, v1: LuaValue, v2: LuaValue) -> LuaResult<bool> {
    if v1.equals(v2.clone())? {
        return Ok(true);
    }

    if let (LuaValue::Table(t1), LuaValue::Table(t2)) = (v1, v2) {
        for pair in t1.clone().pairs::<LuaValue, LuaValue>() {
            let (key, value) = pair?;
            if !equal(_lua, value, t2.get(key)?)? {
                return Ok(false);
            }
        }
        for pair in t2.clone().pairs::<LuaValue, LuaValue>() {
            let (key, _) = pair?;
            if !t1.contains_key(key)? {
                return Ok(false);
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}
