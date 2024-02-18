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
    #[arg(long, short, global = true, default_values_t = [
        String::from("lua51"),
        String::from("lua52"),
        String::from("lua53"),
        String::from("lua54"),
    ])]
    lua_features: Vec<String>,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    Build,
    Test,
    Install,
}

impl Args {
    fn main(&self) -> Result<()> {
        match &self.command {
            Subcommand::Build => {
                self.build(&self.lua_features, false)?;
            }
            Subcommand::Test => {
                self.test(&self.lua_features)?;
            }
            Subcommand::Install => {
                self.build(&self.lua_features, true)?;
                self.install()?;
            }
        }
        Ok(())
    }

    fn build(&self, lua_features: &[String], lib_only: bool) -> Result<()> {
        for feature in lua_features {
            self.build_lib(false, feature)?;
        }
        if lib_only {
            return Ok(());
        }
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build").args(["--package", "lunest"]);
        #[cfg(not(debug_assertions))]
        cmd.arg("--release");
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("build failed");
        }
        Ok(())
    }

    fn build_lib(&self, test: bool, lua_feature: &str) -> Result<()> {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build")
            .args(["--package", "lunest_lib"])
            .args(["--message-format", "json-render-diagnostics"]);
        #[cfg(not(debug_assertions))]
        cmd.arg("--release");
        set_features(&mut cmd, test, lua_feature);
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
            fs::copy(artifact, lunest_shared::utils::dll_path(lua_feature))?;
        }
        if !child.wait()?.success() {
            bail!("build failed");
        }

        Ok(())
    }

    fn test(&self, lua_features: &[String]) -> Result<()> {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("test")
            .args(["--package", "lunest_shared"])
            .arg("--all-features");
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("test failed")
        }

        for feature in lua_features {
            self.test_lib(feature)?;
        }

        Ok(())
    }

    fn test_lib(&self, lua_feature: &str) -> Result<()> {
        self.build_lib(true, lua_feature)?;

        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build")
            .args(["--package", "lua_rt"])
            .args(["--features", "vendored"]);
        set_features(&mut cmd, false, lua_feature);
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("build failed");
        }

        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("test").args(["--package", "lunest_lib"]);
        set_features(&mut cmd, true, lua_feature);
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("test failed")
        }
        Ok(())
    }

    fn install(&self) -> Result<()> {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("install").args(["--path", "."]);
        #[cfg(debug_assertions)]
        cmd.arg("--debug");
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("install failed")
        }
        Ok(())
    }
}

fn set_features(cmd: &mut Command, test: bool, lua_feature: &str) {
    cmd.args(["--no-default-features", "--features", lua_feature]);
    if test {
        cmd.args(["--features", "test"]);
    }
}

fn get_artifact(json: &str) -> Result<PathBuf> {
    let json: serde_json::Value = serde_json::from_str(json)?;
    let Some(filenames) = json.get("filenames") else {
        bail!("'filenames' field not found");
    };
    let lib_name = format!("{DLL_PREFIX}lunest_lib{DLL_SUFFIX}");
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
    println!(
        "\n──────────── {}",
        lunest_shared::utils::command_to_string(cmd)
    );
}
