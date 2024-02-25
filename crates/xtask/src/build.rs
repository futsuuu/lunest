use std::{
    fs,
    env::consts::*,
    ffi::OsStr,
    path::PathBuf,
    io::{BufRead, BufReader},
    process::Stdio,
};

use anyhow::{bail, Result};

use crate::{cargo, sep, Common, Lua};

#[derive(clap::Parser)]
pub struct Build {
    #[command(flatten)]
    pub common: Common,
    #[arg(long, short)]
    pub release: bool,
    #[arg(long)]
    pub debug: bool,
    #[arg(long)]
    pub target: Option<String>,
}

impl Build {
    pub fn build(&self) -> Result<()> {
        self.build_libs(false)?;
        let mut cmd = cargo!("build");
        cmd.args(["--package", "lunest"]);
        if self.release || (cfg!(not(debug_assertions)) && !self.debug) {
            cmd.arg("--release");
        }
        if let Some(target) = self.target.as_ref() {
            cmd.args(["--target", target]);
        }
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("build failed");
        }
        Ok(())
    }

    pub fn install(&self) -> Result<()> {
        self.build_libs(false)?;
        let mut cmd = cargo!("install");
        cmd.args(["--package", "lunest"])
            .args(["--path", "."]);
        if self.debug {
            cmd.arg("--debug");
        }
        if let Some(target) = self.target.as_ref() {
            cmd.args(["--target", target]);
        }
        sep(&cmd);
        if cmd.status()?.success() {
            bail!("install failed");
        }
        Ok(())
    }

    pub fn build_libs(&self, test: bool) -> Result<()> {
        for lua in &self.common.lua {
            self.build_lib(lua, test)?;
        }
        Ok(())
    }

    fn build_lib(&self, lua: &Lua, test: bool) -> Result<()> {
        let mut cmd = cargo!("build");
        cmd.args(["--package", "lunest_lib"])
            .args(["--message-format", "json-render-diagnostics"])
            .args(["--no-default-features", "--features", lua.into()]);
        if test {
            cmd.args(["--features", "test"]);
        }
        if let Some(target) = self.target.as_ref() {
            cmd.args(["--target", target]);
        }
        if self.release || (cfg!(not(debug_assertions)) && !self.debug) {
            cmd.arg("--release");
        }

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
            fs::copy(artifact, lunest_shared::utils::dll_path(lua.into()))?;
        }
        if !child.wait()?.success() {
            bail!("build failed");
        }

        Ok(())
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
