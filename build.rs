use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    println!("cargo:rerun-if-changed=lua");

    let zip = {
        let path = out_dir.join("lua.zip");
        if path.exists() {
            fs::remove_file(&path)?;
        }
        File::options().write(true).create(true).open(&path)?
    };
    let mut zip = zip::ZipWriter::new(zip);
    let file_opts = zip::write::FileOptions::default();

    for entry in fs::read_dir("lua")?.filter_map(Result::ok) {
        let path = entry.path();
        zip.start_file(path.file_name().unwrap().to_str().unwrap(), file_opts)?;
        zip.write_all(&fs::read(path)?)?;
    }

    zip.finish()?;

    Ok(())
}
