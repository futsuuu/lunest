use std::{
    env::consts::*,
    fs,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
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

        let dist = project_root::get_project_root()?.join("dist");
        fs::create_dir_all(&dist)?;
        let mut name = String::from("lunest");
        if let Some(target) = self.target.as_ref() {
            name += "-";
            name += target;
        }
        name += EXE_SUFFIX;
        let dist = dist.join(name);
        for_each_artifact(&mut cmd, |artifact| {
            if artifact.name == "lunest" && artifact.typ == ArtifactType::Exe {
                fs::copy(artifact.path, &dist)?;
            }
            Ok(())
        })?;

        Ok(())
    }

    pub fn install(&self) -> Result<()> {
        self.build_libs(false)?;
        let mut cmd = cargo!("install");
        cmd.args(["--package", "lunest"]).args(["--path", "."]);
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
            .arg("--no-default-features")
            .args(["--features", lua.into()]);
        if test {
            cmd.args(["--features", "test"]);
        }
        if let Some(target) = self.target.as_ref() {
            cmd.args(["--target", target]);
        }
        if self.release || (cfg!(not(debug_assertions)) && !self.debug) {
            cmd.arg("--release");
        }

        for_each_artifact(&mut cmd, |artifact| {
            if artifact.name == "lunest_lib" && artifact.typ == ArtifactType::Dll {
                fs::copy(artifact.path, lunest_shared::utils::dll_path(lua.into()))?;
            }
            Ok(())
        })?;

        Ok(())
    }
}

fn for_each_artifact<F>(cmd: &mut Command, f: F) -> Result<()>
where
    F: Fn(Artifact) -> Result<()>,
{
    use serde_json::Value;
    cmd.args(["--message-format", "json-render-diagnostics"]);
    sep(&cmd);
    let mut child = cmd.stdout(Stdio::piped()).spawn()?;
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut buffer = String::new();
        if reader.read_line(&mut buffer)? == 0 {
            break;
        }
        let json: Value = serde_json::from_str(&buffer)?;
        let Some(name) = json.get("target").and_then(|v| v.get("name")) else {
            continue;
        };
        let name = name.as_str().unwrap();
        if let Some(exe) = json.get("executable").and_then(Value::as_str) {
            f(Artifact {
                name,
                path: Path::new(exe),
                typ: ArtifactType::Exe,
            })?;
            continue;
        }
        let Some(filenames) = json.get("filenames").and_then(Value::as_array) else {
            continue;
        };
        for file in filenames {
            let path = Path::new(file.as_str().unwrap());
            let filename = path.file_name().unwrap().to_str().unwrap();
            if filename.starts_with(DLL_PREFIX) && filename.ends_with(DLL_SUFFIX) {
                f(Artifact {
                    name,
                    path,
                    typ: ArtifactType::Dll,
                })?;
            }
        }
    }
    if !child.wait()?.success() {
        bail!("build failed");
    }
    Ok(())
}

struct Artifact<'a> {
    name: &'a str,
    path: &'a Path,
    typ: ArtifactType,
}

#[derive(PartialEq)]
enum ArtifactType {
    Exe,
    Dll,
}
