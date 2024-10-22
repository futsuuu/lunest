mod config;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
};

use anyhow::{Context, Result};
use clap::Parser;
use notify::Watcher;
use serde::Deserialize;
use yansi::Paint;

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
    let temp_dir = env::temp_dir().join("lunest");
    if temp_dir.try_exists()? {
        fs::remove_dir_all(&temp_dir)?;
        fs::create_dir(&temp_dir)?;
    } else {
        fs::create_dir_all(&temp_dir)?;
    }

    let config = Config::read(&root_dir)?;
    let (profile_name, profile) = config.profile(profile.as_deref())?;
    println!("run with profile '{}'\n", profile_name.bold());

    let init_lua = temp_dir.join("init.lua");
    let result_dir = temp_dir.join("result");
    fs::create_dir(&result_dir)?;
    setup_init_lua(
        &init_lua,
        &root_dir,
        &profile.target_files(&root_dir)?,
        &result_dir,
        profile.init_file()?,
    )?;

    let mut process = {
        let mut cmd = profile.lua_command()?;
        cmd.arg(&init_lua);
        cmd.spawn().with_context(|| {
            format!(
                "failed to spawn process `{}`",
                cmd.get_program().to_str().unwrap()
            )
        })?
    };

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        if let Ok(event) = event {
            let _ = tx.send(event);
        }
    })?;
    watcher.watch(&result_dir, notify::RecursiveMode::NonRecursive)?;

    let mut results = Vec::new();

    loop {
        if let Some(status) = process.try_wait()? {
            match status.code() {
                Some(0) => break,
                Some(n) => anyhow::bail!("spawned process exited with status code {n}"),
                None => anyhow::bail!("spawned process terminated by signal"),
            }
        }

        use notify::event::*;
        let paths = match rx.try_recv() {
            Ok(e) if e.kind == EventKind::Access(AccessKind::Close(AccessMode::Write)) => e.paths,
            Err(mpsc::TryRecvError::Disconnected) => break,
            _ => continue,
        };
        for path in paths {
            let result: TestResult = serde_json::from_str(&fs::read_to_string(path)?)?;
            result.print();
            results.push(result);
        }
    }

    drop(watcher);
    fs::remove_dir_all(&temp_dir)?;

    println!(
        "\nsuccess: {}, error: {}",
        results.iter().filter(|r| r.success()).count().green(),
        results.iter().filter(|r| !r.success()).count().red(),
    );
    if results.iter().any(|r| !r.success()) {
        process::exit(1);
    }
    Ok(())
}

fn wrapper_cmd(save: Option<PathBuf>) -> Result<()> {
    let source = concat!(
        "-- Code generated by `lunest wrapper`. DO NOT EDIT.\n",
        "---@diagnostic disable\n",
        include_str!("wrapper.lua")
    );
    if let Some(path) = save {
        anyhow::ensure!(!path.exists(), "file already exists");
        fs::write(path, source)?;
    } else {
        print!("{}", source);
    }
    Ok(())
}

fn setup_init_lua(
    path: &Path,
    root_dir: &Path,
    target_files: &[PathBuf],
    result_dir: &Path,
    init_file: Option<&Path>,
) -> Result<()> {
    let files: String = target_files.iter().fold(String::new(), |acc, p| {
        let name = p.strip_prefix(root_dir).unwrap_or(p);
        format!(
            "{acc}{{ name = \"{}\", path = \"{}\" }}, ",
            name.display().to_string().replace('\\', "/"),
            p.display().to_string().replace('\\', r"\\"),
        )
    });
    let contents = include_str!(concat!(env!("OUT_DIR"), "/main.lua"))
        .replace(
            "local TARGET_FILES\n",
            &format!("local TARGET_FILES = {{ {files} }}\n"),
        )
        .replace(
            "local RESULT_DIR\n",
            &format!(
                "local RESULT_DIR = \"{}\"\n",
                result_dir.display().to_string().replace('\\', r"\\")
            ),
        );
    let contents = if let Some(path) = init_file {
        contents.replace(
            "local INIT_FILE\n",
            &format!(
                "local INIT_FILE = \"{}\"\n",
                path.display().to_string().replace('\\', r"\\"),
            ),
        )
    } else {
        contents
    };
    fs::write(path, contents)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct TestResult {
    title: Vec<String>,
    error: Option<TestError>,
}

#[derive(Debug, Deserialize)]
enum TestError {
    Msg(String),
}

impl TestResult {
    fn success(&self) -> bool {
        self.error.is_none()
    }

    fn print(&self) {
        print!(
            "{}{} ",
            self.title.join(&" :: ".dim().to_string()),
            ":".dim()
        );
        let Some(err) = &self.error else {
            println!("{}", "OK".green().bold());
            return;
        };
        println!("{}", "ERR".red().bold());
        match err {
            TestError::Msg(msg) => println!("{msg}"),
        }
    }
}
