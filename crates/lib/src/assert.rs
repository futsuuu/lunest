mod equal;
mod stringify;

use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use anyhow::Result;
use mlua::prelude::*;

use super::CalledPos;
use equal::equal;
use stringify::stringify;

pub fn assert(
    _lua: &Lua,
    (pos, v, message): (CalledPos, LuaValue, Option<String>),
) -> LuaResult<()> {
    _assert(
        &pos,
        v != LuaValue::Nil && v != LuaValue::Boolean(false),
        message.as_ref().map(|s| s.as_str()),
    )
}

pub fn assert_eq(
    lua: &Lua,
    (pos, a, b, message): (CalledPos, LuaValue, LuaValue, Option<String>),
) -> LuaResult<()> {
    if !equal(lua, a.clone(), b.clone())? {
        let message = message.unwrap_or("two values are not equal".to_string());
        return _assert(
            &pos,
            false,
            Some(&format!(
                "{message}\nleft:  {}\nright: {}",
                stringify(a),
                stringify(b)
            )),
        );
    }
    Ok(())
}

pub fn assert_ne(
    lua: &Lua,
    (pos, a, b, message): (CalledPos, LuaValue, LuaValue, Option<String>),
) -> LuaResult<()> {
    if equal(lua, a.clone(), b.clone())? {
        let message = message.unwrap_or("two values are equal".to_string());
        return _assert(
            &pos,
            false,
            Some(&format!(
                "{message}\nleft:  {}\nright: {}",
                stringify(a),
                stringify(b)
            )),
        );
    }
    Ok(())
}

fn _assert(pos: &CalledPos, b: bool, message: Option<&str>) -> LuaResult<()> {
    if b {
        return Ok(());
    }
    let mut err = String::from("assertion failed!\n");
    if let Ok(code) = show_called_pos(&pos) {
        err += "\n";
        err += &code;
    }
    if let Some(msg) = message {
        err += "\n";
        err += msg;
        err += "\n";
    }
    Err(LuaError::runtime(err))
}

fn show_called_pos(pos: &CalledPos) -> Result<String> {
    let file = File::open(&pos.path)?;
    let mut reader = BufReader::new(file);
    let mut code = String::new();
    let mut line: usize = 1;
    loop {
        if line.abs_diff(pos.line) < 5 {
            code += &format!("{line: >8} ");
            code += if pos.line == line { "╪" } else { "│" };
            if reader.read_line(&mut code)? == 0 {
                code += "\n";
                break;
            }
        } else {
            if reader.read_line(&mut String::new())? == 0 {
                break;
            }
        }
        line += 1;
    }
    Ok(code)
}
