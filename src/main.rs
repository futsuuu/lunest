use std::{
    env, fs,
    io::{stdout, Cursor, Write},
    path::Path,
    process::{exit, Command},
};

use anyhow::{Context as _, Result};
use clap::Parser as _;

fn main() -> Result<()> {
    let args = Args::parse();
    let data_dir = dirs::data_dir()
        .context("cannot get the data directory")?
        .join("lunest");
    extract_archive_if_needed(&data_dir)?;
    let status = Command::new(&args.lua_cmd[0])
        .args(&args.lua_cmd[1..])
        .arg(data_dir.join("lunest.lua"))
        .arg("run")
        .status()?;
    exit(status.code().unwrap_or(1));
}

#[derive(clap::Parser)]
struct Args {
    #[arg(num_args = 1.., required = true, allow_hyphen_values = true)]
    lua_cmd: Vec<String>,
}

fn extract_archive_if_needed(data_dir: &Path) -> Result<()> {
    if data_dir.exists() {
        let self_modified = fs::metadata(env::current_exe()?)?.modified()?;
        let archive_modified = fs::metadata(data_dir)?.modified()?;
        if archive_modified > self_modified {
            return Ok(());
        }
        fs::remove_dir_all(data_dir)?;
    }
    fs::create_dir_all(data_dir)?;
    print!("extracting archives into {}", data_dir.display());
    stdout().flush()?;
    let zip_file = include_bytes!(concat!(env!("OUT_DIR"), "/lua.zip"));
    let mut zip = zip::ZipArchive::new(Cursor::new(zip_file.as_slice()))?;
    zip.extract(data_dir)?;
    println!(": done\n");
    Ok(())
}
