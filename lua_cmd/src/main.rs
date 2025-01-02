use std::{ffi::OsString, path::PathBuf};

fn main() {
    let args = match Args::parse_from(std::env::args_os()) {
        Ok(args) => args,
        Err(e) => {
            println!("{e}\n{}", help());
            std::process::exit(1);
        }
    };
    let Err(e) = main_inner(args) else {
        return;
    };
    match e {
        mlua::Error::RuntimeError(msg) => println!("lua error: {msg}"),
        _ => println!("lua error: {e}"),
    }
    std::process::exit(1);
}

fn main_inner(args: Args) -> mlua::Result<()> {
    if args.version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    }
    let lua = unsafe { mlua::Lua::unsafe_new() };
    for e in &args.execute {
        lua.load(e.as_encoded_bytes()).exec()?;
    }
    if let Some(s) = &args.script {
        lua.globals()
            .set("arg", lua.create_table_from(args.lua_arg())?)?;
        lua.load(s.as_path()).exec()?;
    }
    Ok(())
}

fn help() -> String {
    let exe = std::env::current_exe().map_or_else(
        |_| env!("CARGO_BIN_NAME").to_string(),
        |p| p.display().to_string(),
    );
    format!(
        "usage: {exe} [options] [script [args]].
Available options are:
  -e stat  execute string 'stat'
  -v       show version information
  --       stop handling options"
    )
}

#[cfg_attr(test, derive(Debug, PartialEq))]
struct Args {
    raw: Vec<OsString>,
    script: Option<PathBuf>,
    args: Vec<OsString>,
    execute: Vec<OsString>,
    version: bool,
}

impl Args {
    fn parse_from(args: impl IntoIterator<Item = impl Into<OsString>>) -> Result<Self, String> {
        let raw = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let mut args = raw.iter().filter_map(|s| s.to_str());
        args.next();
        let mut execute = Vec::new();
        let mut version = false;
        let mut script = None;
        while let Some(arg) = args.next() {
            match arg {
                "-e" => {
                    execute.push(
                        args.next()
                            .ok_or("'stat' does not specified".to_string())?
                            .into(),
                    );
                }
                "-v" => {
                    version = true;
                }
                "--" => {
                    script = args.next().map(PathBuf::from);
                    break;
                }
                s => {
                    script = Some(PathBuf::from(s));
                    break;
                }
            }
        }
        let args = args.map(Into::into).collect();

        Ok(Self {
            raw,
            script,
            args,
            execute,
            version,
        })
    }

    fn lua_arg(&self) -> impl IntoIterator<Item = (isize, &std::ffi::OsStr)> {
        let offset = self.raw.len() as isize - self.args.len() as isize - 1;
        self.raw
            .iter()
            .enumerate()
            .map(move |(i, arg)| (i as isize - offset, arg.as_os_str()))
    }
}

#[cfg(test)]
mod args_tests {
    use super::*;

    #[test]
    fn parse() {
        let args: Vec<_> = [
            "foo",
            "-e",
            "print('hello')",
            "-v",
            "-e",
            "a = 1",
            "--",
            "-e",
            "a",
            "b",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        assert_eq!(
            Args {
                raw: args.clone(),
                script: Some(PathBuf::from("-e")),
                args: vec!["a".into(), "b".into()],
                execute: vec!["print('hello')".into(), "a = 1".into()],
                version: true,
            },
            Args::parse_from(args).unwrap(),
        );
    }

    #[test]
    fn lua_arg() {
        assert_eq!(
            [
                (-3, "lua"),
                (-2, "-e"),
                (-1, "sin = math.sin"),
                (0, "script"),
                (1, "a"),
                (2, "b"),
            ]
            .map(|(i, a)| (i, std::ffi::OsStr::new(a)))
            .to_vec(),
            Args::parse_from(["lua", "-e", "sin = math.sin", "script", "a", "b"])
                .unwrap()
                .lua_arg()
                .into_iter()
                .collect::<Vec<_>>(),
        );
    }
}
