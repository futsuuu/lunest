use std::cmp::Ordering;

use lunest_macros::lua_module_test;
#[cfg(feature = "test")]
use mlua::chunk;
use mlua::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

#[cfg(test)]
use crate::tests::lua_eval;

pub fn stringify(v: LuaValue) -> String {
    let s = match v {
        LuaValue::Nil => format!("nil"),
        LuaValue::Boolean(b) => b.to_string(),
        LuaValue::Integer(i) => i.to_string(),
        LuaValue::Number(n) => n.to_string(),
        LuaValue::String(s) => format!("'{}'", s.to_string_lossy()),
        LuaValue::Table(t) => {
            let mut result = String::from("{ ");

            for (key, value) in sort_table(t) {
                result += &(table_key(&stringify(key)) + " = " + &stringify(value) + ", ");
            }
            if let Some(stripped) = result.strip_suffix(", ") {
                stripped.to_string() + " }"
            } else {
                "{}".to_string()
            }
        }
        LuaValue::Function(_) => format!("<function>"),
        LuaValue::Thread(_) => format!("<thread>"),
        LuaValue::LightUserData(_) => format!("<userdata>"),
        LuaValue::UserData(_) => format!("<userdata>"),
        LuaValue::Error(e) => e.to_string(),
    };
    s
}

fn sort_table(t: LuaTable) -> Vec<(LuaValue, LuaValue)> {
    let mut t = t
        .pairs()
        .filter_map(Result::ok)
        .collect::<Vec<(LuaValue, LuaValue)>>();
    t.sort_by(|a, b| {
        let a = &a.0;
        let b = &b.0;
        if let (Some(n1), Some(n2)) = (as_number(a), as_number(b)) {
            return n1.total_cmp(&n2);
        }
        if let (Some(b1), Some(b2)) = (a.as_boolean(), b.as_boolean()) {
            return b1.cmp(&b2);
        }
        if let (Some(s1), Some(s2)) = (
            a.as_string_lossy().map(|s| s.to_string()),
            b.as_string_lossy().map(|s| s.to_string()),
        ) {
            return s1.cmp(&s2);
        }
        Ordering::Equal
    });
    t
}

fn as_number(value: &LuaValue) -> Option<LuaNumber> {
    match value {
        LuaValue::Number(n) => Some(*n),
        LuaValue::Integer(i) => Some(*i as LuaNumber),
        _ => None,
    }
}

fn table_key(s: &str) -> String {
    let s = s.strip_suffix('\'').unwrap().strip_prefix('\'').unwrap();
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new("^[A-Za-z_][A-Za-z0-9_]*$").unwrap());
    let s = match s {
        "and" | "break" | "do" | "else" | "elseif" | "end" | "false" | "for"
        | "function" | "goto" | "if" | "in" | "local" | "nil" | "not" | "or"
        | "repeat" | "return" | "then" | "true" | "until" | "while" => s,
        s if RE.is_match(s) => {
            return s.to_string();
        }
        _ => s,
    };
    format!("['{s}']")
}

#[lua_module_test(lua_eval)]
fn normal(lua: &Lua) -> LuaResult<()> {
    assert_eq!(String::from("nil"), stringify(LuaValue::Nil));
    assert_eq!(String::from("true"), stringify(LuaValue::Boolean(true)));
    assert_eq!(String::from("1"), stringify(LuaValue::Number(1.0)));
    assert_eq!(String::from("1.1"), stringify(LuaValue::Number(1.1)));
    assert_eq!(
        String::from("'hello'"),
        stringify(LuaValue::String(lua.create_string("hello")?))
    );
    Ok(())
}

#[lua_module_test(lua_eval)]
fn table(lua: &Lua) -> LuaResult<()> {
    assert_eq!(
        String::from("{ ['!'] = 'hello', a = {}, b = 2, ['return'] = 0.5 }"),
        stringify(
            lua.load(chunk! {
                return { b = 2, a = {}, ["!"] = "hello", ["return"] = 0.5 }
            })
            .eval::<LuaTable>()?
            .into_lua(lua)?
        )
    );
    Ok(())
}
