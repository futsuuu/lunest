mod bridge;
mod config;

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{cursor, style::Stylize};

/// Lua testing framework
#[derive(Debug, Parser)]
#[command(version, about)]
enum Args {
    /// Run tests
    #[command(visible_alias = "r")]
    Run {
        /// Run tests with the specified profile
        #[arg(long, short, value_delimiter = ',')]
        profile: Vec<String>,
        /// Run tests with the profiles in the specified group
        #[arg(long, short, value_delimiter = ',')]
        group: Vec<String>,
    },

    /// Print wrapper Lua code used for in-source testing
    Wrapper {
        /// Write code into the specified file
        #[arg(long, short, value_name = "FILE")]
        save: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args {
        Args::Run { profile, group } => run_cmd(profile, group)?,
        Args::Wrapper { save } => wrapper_cmd(save)?,
    }
    Ok(())
}

#[test]
fn test_lua() -> Result<()> {
    std::env::set_current_dir("..")?;
    #[cfg(feature = "luacmds-all")]
    run_cmd(vec![], vec!["all".into()])?;
    #[cfg(not(feature = "luacmds-all"))]
    run_cmd(vec![], vec![])?;
    Ok(())
}

fn run_cmd(profiles: Vec<String>, groups: Vec<String>) -> Result<()> {
    let root_dir = std::env::current_dir()?;
    let config = config::Config::read(&root_dir)?;
    let profiles = {
        let mut ps = indexmap::IndexMap::new();
        for profile in &profiles {
            let (s, p) = config.profile(Some(profile))?;
            ps.insert(s, p);
        }
        for group in &groups {
            ps.extend(config.group(group)?);
        }
        if ps.is_empty() {
            let (s, p) = config.profile(None)?;
            ps.insert(s, p);
        }
        ps
    };

    let mut has_error = false;
    for (i, (profile_name, profile)) in profiles.iter().enumerate() {
        if i != 0 {
            println!();
        }
        if !run(profile_name, profile, &root_dir)? {
            has_error = true;
        }
    }
    if has_error {
        std::process::exit(1);
    }
    Ok(())
}

fn run(profile_name: &str, profile: &config::Profile, root_dir: &Path) -> Result<bool> {
    println!("run with profile '{}'", profile_name.bold());

    let temp_dir = tempfile::TempDir::with_prefix(env!("CARGO_CRATE_NAME"))?;
    let mut bridge = bridge::Bridge::new(temp_dir.path())?;
    let mut process = {
        let main_lua = temp_dir.path().join("main.lua");
        std::fs::write(
            &main_lua,
            bridge.overwrite_main_lua(
                include_str!(concat!(env!("OUT_DIR"), "/main.lua")),
                root_dir,
                &profile.target_files(root_dir)?,
                profile.init_file()?,
            ),
        )?;
        let mut cmd = profile.lua_command(temp_dir.path())?;
        cmd.arg(&main_lua);
        println!("spawn {}", display_command(&cmd));
        cmd.spawn().with_context(|| {
            format!(
                "failed to spawn process `{}`",
                cmd.get_program().to_str().unwrap()
            )
        })?
    };

    let mut results = Vec::new();
    println!();

    loop {
        if let Some(message) = bridge.read()? {
            match message {
                bridge::Message::TestFinished(t) => {
                    println!("{t}");
                    results.push(t);
                }
                bridge::Message::TestStarted(t) => {
                    print!("{t}{}", cursor::MoveToColumn(0));
                    let _ = std::io::stdout().flush();
                }
            }
        } else if let Some(status) = process.try_wait()? {
            match status.code() {
                Some(0) => break,
                Some(n) => anyhow::bail!("spawned process exited with status code {n}"),
                None => anyhow::bail!("spawned process terminated by signal"),
            }
        }
    }

    let (success, error): (Vec<_>, Vec<_>) = results.iter().partition(|r| r.success());
    println!(
        "\nsuccess: {}, error: {}",
        success.len().to_string().green(),
        error.len().to_string().red(),
    );
    Ok(error.is_empty())
}

fn wrapper_cmd(save: Option<PathBuf>) -> Result<()> {
    let source = concat!(
        "-- Code generated by `lunest wrapper`. DO NOT EDIT.\n",
        "---@diagnostic disable\n",
        include_str!("../../module/lunest/wrapper.lua")
    );
    if let Some(path) = save {
        anyhow::ensure!(!path.exists(), "file already exists");
        std::fs::write(path, source)?;
    } else {
        print!("{}", source);
    }
    Ok(())
}

fn display_command(cmd: &std::process::Command) -> String {
    fn fmt_osstr(s: &std::ffi::OsStr) -> String {
        let s = s.to_str().unwrap_or("(invalid UTF-8)");
        if s.contains(' ') {
            let s = s.replace('"', "\\\"");
            format!("\"{s}\"")
        } else {
            s.into()
        }
    }
    let mut s = String::new();
    s += &fmt_osstr(cmd.get_program()).cyan().to_string();
    for a in cmd.get_args() {
        s += " ";
        s += &fmt_osstr(a).magenta().to_string();
    }
    s
}
