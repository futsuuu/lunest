use std::{env, fs, path::PathBuf};

use anyhow::Result;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let lua_dir = env::current_dir()?.join("lua");
    println!("cargo:rerun-if-changed={}", lua_dir.display());
    fs::write(
        out_dir.join("main.lua"),
        bundler::bundle(&lua_dir.join("lunest.lua"))?,
    )?;
    Ok(())
}
