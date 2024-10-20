use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread,
};

use anyhow::{Context, Result};
use notify::Watcher;
use serde::Deserialize;
use yansi::Paint;

fn main() -> Result<()> {
    let root_dir = env::current_dir()?;
    let temp_dir = env::temp_dir().join("lunest");
    if temp_dir.try_exists()? {
        fs::remove_dir_all(&temp_dir)?;
        fs::create_dir(&temp_dir)?;
    } else {
        fs::create_dir_all(&temp_dir)?;
    }

    let config = read_config(&root_dir)?;
    let init_lua = temp_dir.join("init.lua");
    let result_dir = temp_dir.join("result");
    fs::create_dir(&result_dir)?;
    setup_init_lua(
        &init_lua,
        &root_dir,
        &expand_glob(&root_dir, &config.files)?,
        &result_dir,
    )?;

    let handle = thread::spawn(move || spawn_lua(&config.lua, &init_lua));

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        if let Ok(event) = event {
            let _ = tx.send(event);
        }
    })?;
    watcher.watch(&result_dir, notify::RecursiveMode::NonRecursive)?;

    let mut results = Vec::new();
    loop {
        if handle.is_finished() {
            break;
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
    if let Ok(result) = handle.join() {
        result?;
    }
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

fn spawn_lua(command: &[String], file_path: &Path) -> Result<()> {
    let mut cmd = process::Command::new(command.first().context("command is empty")?);
    cmd.args(command.get(1..).unwrap_or_default())
        .arg(file_path);
    let status = cmd
        .spawn()
        .with_context(|| format!("failed to spawn process `{}`", command[0]))?
        .wait()?;
    anyhow::ensure!(status.success());
    Ok(())
}

fn setup_init_lua(
    path: &Path,
    root_dir: &Path,
    target_files: &[PathBuf],
    result_dir: &Path,
) -> Result<()> {
    let files: String = target_files.iter().fold(String::new(), |acc, p| {
        let name = p.strip_prefix(root_dir).unwrap_or(p);
        format!(
            "{acc}{{ name = \"{}\", path = \"{}\" }}, ",
            name.display().to_string().replace('\\', "/"),
            p.display().to_string().replace('\\', r"\\"),
        )
    });
    let contents = include_str!("init.lua")
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
    fs::write(path, contents)?;
    Ok(())
}

fn read_config(root_dir: &Path) -> Result<Config> {
    let config_path = root_dir.join(".lunest").join("config.toml");
    let config = match fs::read_to_string(config_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(Default::default());
        }
        r => r?,
    };
    let config = toml::from_str(&config)?;
    Ok(config)
}

fn expand_glob(
    root_dir: impl AsRef<Path>,
    patterns: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Vec<PathBuf>> {
    let root = root_dir.as_ref();
    let set = {
        let mut builder = globset::GlobSet::builder();
        for pat in patterns {
            builder.add(globset::Glob::new(pat.as_ref())?);
        }
        builder.build()?
    };
    let mut r = Vec::new();
    for entry in walkdir::WalkDir::new(root).sort_by_file_name() {
        let entry = entry?;
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_file() && set.is_match(entry.path().strip_prefix(root).unwrap()) {
            r.push(entry.into_path());
        }
    }
    Ok(r)
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Config {
    lua: Vec<String>,
    files: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lua: vec!["lua".into()],
            files: vec!["{lua,src}/**/*.lua".into()],
        }
    }
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
