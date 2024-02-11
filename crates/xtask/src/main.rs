use std::{
    env::consts::*,
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::{bail, Result};
use clap::Parser as _;

fn main() -> Result<()> {
    let args = Args::parse();
    args.main()?;
    Ok(())
}

#[derive(clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    Build,
    Test,
}

impl Args {
    fn main(&self) -> Result<()> {
        match &self.command {
            Subcommand::Build => {
                self.build(false)?;
            }
            Subcommand::Test => {
                self.test()?;
            }
        }
        Ok(())
    }

    fn build(&self, test: bool) -> Result<()> {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.args(["build", "--message-format", "json-render-diagnostics"]);
        set_features(&mut cmd, test);
        sep(&cmd);

        let mut child = cmd.stdout(Stdio::piped()).spawn()?;
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        loop {
            let mut buffer = String::new();
            if reader.read_line(&mut buffer)? == 0 {
                break;
            }
            let Ok(artifact) = get_artifact(&buffer) else {
                continue;
            };
            fs::copy(artifact, shared::dll_path())?;
        }
        let status = child.wait()?;
        if !status.success() {
            bail!("build failed");
        }

        Ok(())
    }

    fn test(&self) -> Result<()> {
        self.build(true)?;

        let mut cmd = Command::new(env!("CARGO"));
        cmd.args(["build", "--package", "lua_rt", "--features", "vendored"]);
        set_features(&mut cmd, false);
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("build failed");
        }

        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("test");
        set_features(&mut cmd, true);
        sep(&cmd);
        cmd.status()?;
        Ok(())
    }
}

fn set_features(cmd: &mut Command, test: bool) {
    cmd.args([
        "--no-default-features",
        "--features",
        macros::lua_feature!(),
    ]);
    if test {
        cmd.args(["--features", "test"]);
    }
}

fn get_artifact(json: &str) -> Result<PathBuf> {
    let json: serde_json::Value = serde_json::from_str(json)?;
    let Some(filenames) = json.get("filenames") else {
        bail!("'filenames' field not found");
    };
    let lib_name = format!("{DLL_PREFIX}lunest{DLL_SUFFIX}");
    let filenames: Vec<String> = serde_json::from_value(filenames.clone())?;
    for filename in filenames {
        let path = PathBuf::from(filename);
        if Some(OsStr::new(&lib_name)) == path.file_name() {
            return Ok(path);
        }
    }
    bail!("not found");
}

fn sep(cmd: &Command) {
    println!("\n──────────── {}", shared::command_to_string(cmd));
}
