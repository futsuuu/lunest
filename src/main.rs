mod bridge;
mod config;

use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process,
};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{cursor, style::Stylize};

use config::Config;

/// Lua testing framework
#[derive(Debug, Parser)]
#[command(version, about)]
enum Args {
    /// Run tests
    #[command(visible_alias = "r")]
    Run {
        /// Run tests with the specified profile
        #[arg(long, short)]
        profile: Option<String>,
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
        Args::Run { profile } => run_cmd(profile)?,
        Args::Wrapper { save } => wrapper_cmd(save)?,
    }
    Ok(())
}

fn run_cmd(profile: Option<String>) -> Result<()> {
    let root_dir = env::current_dir()?;
    let temp_dir = {
        let dir = env::temp_dir().join(format!("lunest{:X}", process::id()));
        if dir.try_exists()? {
            fs::remove_dir_all(&dir)?;
            fs::create_dir(&dir)?;
        } else {
            fs::create_dir_all(&dir)?;
        }
        dir
    };

    let mut bridge = bridge::Bridge::new(&temp_dir)?;

    let config = Config::read(&root_dir)?;
    let (profile_name, profile) = config.profile(profile.as_deref())?;
    println!("run with profile '{}'\n", profile_name.bold());

    let mut process = {
        let main_lua = temp_dir.join("main.lua");
        fs::write(
            &main_lua,
            bridge.overwrite_main_lua(
                include_str!(concat!(env!("OUT_DIR"), "/main.lua")),
                &root_dir,
                &profile.target_files(&root_dir)?,
                profile.init_file()?,
            ),
        )?;
        let mut cmd = profile.lua_command()?;
        cmd.arg(&main_lua);
        cmd.spawn().with_context(|| {
            format!(
                "failed to spawn process `{}`",
                cmd.get_program().to_str().unwrap()
            )
        })?
    };

    let mut results = Vec::new();

    loop {
        if let Some(message) = bridge.read()? {
            match message {
                bridge::Message::TestFinished(t) => {
                    println!("{t}");
                    results.push(t);
                }
                bridge::Message::TestStarted(t) => {
                    print!("{t}{}", cursor::MoveToColumn(0));
                    let _ = io::stdout().flush();
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

    fs::remove_dir_all(&temp_dir)?;

    let (success, error): (Vec<_>, Vec<_>) = results.iter().partition(|r| r.success());
    println!(
        "\nsuccess: {}, error: {}",
        success.len().to_string().green(),
        error.len().to_string().red(),
    );
    if !error.is_empty() {
        process::exit(1);
    }
    Ok(())
}

fn wrapper_cmd(save: Option<PathBuf>) -> Result<()> {
    let source = concat!(
        "-- Code generated by `lunest wrapper`. DO NOT EDIT.\n",
        "---@diagnostic disable\n",
        include_str!("../lua/lunest/wrapper.lua")
    );
    if let Some(path) = save {
        anyhow::ensure!(!path.exists(), "file already exists");
        fs::write(path, source)?;
    } else {
        print!("{}", source);
    }
    Ok(())
}
