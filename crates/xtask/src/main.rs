use std::{
    env::{consts::*, current_dir},
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
    sep();
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
            let mut target = current_dir()?.join("lua").join("lunest").join("lunest.so");
            if DLL_EXTENSION == "dll" {
                target.set_extension("dll");
            }
            fs::copy(artifact, target)?;
        }
        let status = child.wait()?;
        if !status.success() {
            bail!("build failed");
        }

        Ok(())
    }

    fn test(&self) -> Result<()> {
        self.build(true)?;
        sep();
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("test");
        set_features(&mut cmd, true);
        cmd.status()?;
        Ok(())
    }
}

fn set_features(cmd: &mut Command, test: bool) {
    cmd.arg("--no-default-features");
    cmd.args([
        "--features",
        #[cfg(feature = "lua51")]
        "lua51",
        #[cfg(feature = "lua52")]
        "lua52",
        #[cfg(feature = "lua53")]
        "lua53",
        #[cfg(feature = "lua54")]
        "lua54",
        #[cfg(feature = "luajit")]
        "luajit",
        #[cfg(feature = "luajit52")]
        "luajit52",
        #[cfg(feature = "luau")]
        "luau",
        #[cfg(feature = "luau-jit")]
        "luau-jit",
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

fn sep() {
    println!("────────────");
}
