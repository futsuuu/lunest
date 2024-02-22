mod equal;
mod stringify;

use mlua::prelude::*;

use equal::equal;
use stringify::stringify;

pub fn assert(_lua: &Lua, (v, message): (LuaValue, Option<String>)) -> LuaResult<()> {
    assert_(
        v != LuaValue::Nil && v != LuaValue::Boolean(false),
        message.as_ref().map(|s| s.as_str()),
    )
}

pub fn assert_eq(lua: &Lua, (a, b): (LuaValue, LuaValue)) -> LuaResult<()> {
    if !equal(lua, a.clone(), b.clone())? {
        return assert_(
            false,
            Some(&format!(
                "two values are not equal
left:  {}
right: {}",
                stringify(a),
                stringify(b)
            )),
        );
    }
    Ok(())
}

pub fn assert_ne(lua: &Lua, (a, b): (LuaValue, LuaValue)) -> LuaResult<()> {
    if equal(lua, a.clone(), b.clone())? {
        return assert_(
            false,
            Some(&format!(
                "two values are equal
left:  {}
right: {}",
                stringify(a),
                stringify(b)
            )),
        );
    }
    Ok(())
}

fn assert_(b: bool, message: Option<&str>) -> LuaResult<()> {
    if !b {
        let mut err = String::from("assertion failed");
        if let Some(msg) = message {
            err += ": ";
            err += msg;
        }
        Err(LuaError::runtime(err))
    } else {
        Ok(())
    }
}
