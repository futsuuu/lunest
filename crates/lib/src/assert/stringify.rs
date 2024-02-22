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
                let key = table_key(&stringify(key));
                let value = &stringify(value);
                result += &(key + " = " + value + ", ");
            }
            if let Some(stripped) = result.strip_suffix(", ") {
                stripped.to_string() + " }"
            } else {
                "{}".to_string()
            }
        }
        LuaValue::Function(_) => format!("<function>"),
        LuaValue::Thread(_) => format!("<thread>"),
        LuaValue::LightUserData(_) => format!("<lightuserdata>"),
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
    t.sort_by(|a, b| compare(&a.0, &b.0));
    t
}

fn compare(a: &LuaValue, b: &LuaValue) -> Ordering {
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
    use Ordering::*;
    if a.is_number() || a.is_integer() {
        Less
    } else if b.is_number() || b.is_integer() {
        Greater
    } else if a.is_boolean() {
        Less
    } else if b.is_boolean() {
        Greater
    } else if a.is_string() {
        Less
    } else if b.is_string() {
        Greater
    } else if a.is_table() && !b.is_table() {
        Less
    } else if b.is_table() && !a.is_table() {
        Greater
    } else if a.is_function() && !b.is_function() {
        Less
    } else if b.is_function() && !a.is_function() {
        Greater
    } else if a.is_light_userdata() && !b.is_light_userdata() {
        Less
    } else if b.is_light_userdata() && !a.is_light_userdata() {
        Greater
    } else if a.is_userdata() && !b.is_userdata() {
        Less
    } else if b.is_userdata() && !a.is_userdata() {
        Greater
    } else {
        Equal
    }
}

fn as_number(value: &LuaValue) -> Option<LuaNumber> {
    match value {
        LuaValue::Number(n) => Some(*n),
        LuaValue::Integer(i) => Some(*i as LuaNumber),
        _ => None,
    }
}

fn table_key(s: &str) -> String {
    let Some(s) = s.strip_suffix('\'').and_then(|s| s.strip_prefix('\'')) else {
        return format!("[{s}]");
    };
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
fn single_value(lua: &Lua) -> LuaResult<()> {
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

#[cfg(feature = "test")]
mod table {
    use super::*;

    #[lua_module_test(lua_eval)]
    fn cmp_integer_number(_lua: &Lua) -> LuaResult<()> {
        assert_eq!(
            Ordering::Greater,
            compare(&LuaValue::Number(2.1), &LuaValue::Integer(2))
        );
        assert_eq!(
            Ordering::Less,
            compare(&LuaValue::Number(1.5), &LuaValue::Integer(2))
        );
        Ok(())
    }

    #[lua_module_test(lua_eval)]
    fn cmp_string_number(lua: &Lua) -> LuaResult<()> {
        assert_eq!(
            Ordering::Greater,
            compare(
                &LuaValue::String(lua.create_string("hello")?),
                &LuaValue::Integer(1)
            )
        );
        Ok(())
    }

    #[lua_module_test(lua_eval)]
    fn sort(lua: &Lua) -> LuaResult<()> {
        assert_eq!(
            String::from("{ [1] = 'world', [1.5] = 'foo', [2] = 'zzz', [3] = 'bar', [false] = -1, ['!'] = 'hello', a = { [1] = 1 }, b = 2, [{}] = {} }"),
            stringify(
                lua.load(chunk! {
                    return {
                        b = 2,
                        ["!"] = "hello",
                        "world",
                        [3] = "bar",
                        [false] = -1,
                        [{}] = {},
                        "zzz",
                        a = { 1 },
                        [1.5] = "foo",
                    }
                })
                .eval::<LuaTable>()?
                .into_lua(lua)?
            )
        );
        Ok(())
    }
}

#[cfg(test)]
mod table_key {
    use super::table_key;

    #[test]
    fn string() {
        assert_eq!(String::from("hello"), table_key("'hello'"));
        assert_eq!(String::from("_1"), table_key("'_1'"));
    }

    #[test]
    fn invalid_string() {
        assert_eq!(String::from("['hello world']"), table_key("'hello world'"));
        assert_eq!(String::from("['1']"), table_key("'1'"));
        assert_eq!(String::from("['a-b']"), table_key("'a-b'"));
    }

    #[test]
    fn non_string() {
        assert_eq!(String::from("[1]"), table_key("1"));
        assert_eq!(String::from("[{ a = 1 }]"), table_key("{ a = 1 }"));
        assert_eq!(String::from("[false]"), table_key("false"));
    }

    #[test]
    fn keyword() {
        assert_eq!(String::from("['return']"), table_key("'return'"));
        assert_eq!(String::from("['end']"), table_key("'end'"));
    }
}
