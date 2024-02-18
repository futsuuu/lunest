use std::{
    env, fs,
    io::{stdout, Cursor, Write},
    path::Path,
    process::{exit, Command},
};

use anyhow::{Context as _, Result};
use lunest_shared::{
    cli::{self, Parser as _},
    config::Config,
};

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let profile = Config::load()?.get_profile(&args.profile)?;
    let data_dir = dirs::data_dir()
        .context("cannot get the data directory")?
        .join("lunest");
    extract_archive_if_needed(&data_dir)?;
    let lua_cmd = profile.get_lua();
    let status = Command::new(&lua_cmd[0])
        .args(&lua_cmd[1..])
        .arg(data_dir.join("lunest.lua"))
        .args({
            let mut args = env::args();
            args.next();
            args.collect::<Vec<String>>()
        })
        .status()?;
    exit(status.code().unwrap_or(1));
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
