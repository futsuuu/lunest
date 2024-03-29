use anyhow::{bail, Result};

use crate::{cargo, sep};

#[derive(clap::Parser)]
pub struct Test {
    #[command(flatten)]
    pub common: crate::Common,
}

impl Test {
    pub fn test(&self) -> Result<()> {
        let mut cmd = cargo!("test");
        cmd.args(["--package", "lunest_shared"])
            .arg("--all-features");
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("test failed");
        }

        self.test_libs()?;
        Ok(())
    }

    fn test_libs(&self) -> Result<()> {
        let build = crate::Build {
            common: self.common.clone(),
            debug: false,
            release: false,
            target: None,
        };
        build.build_libs(true)?;
        for lua in &self.common.lua {
            self.test_lib(&lua)?;
        }
        Ok(())
    }

    fn test_lib(&self, lua: &crate::Lua) -> Result<()> {
        let mut cmd = cargo!("build");
        cmd.args(["--package", "lua_rt"])
            .arg("--no-default-features")
            .args(["--features", "vendored"])
            .args(["--features", lua.into()]);
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("build failed");
        }

        let mut cmd = cargo!("test");
        cmd.args(["--package", "lunest_lib"])
            .arg("--no-default-features")
            .args(["--features", lua.into()])
            .args(["--features", "test"]);
        sep(&cmd);
        if !cmd.status()?.success() {
            bail!("test failed");
        }
        Ok(())
    }
}
