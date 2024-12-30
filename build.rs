fn main() -> std::io::Result<()> {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let lua_dir = std::env::current_dir()?.join("lua");
    println!("cargo:rerun-if-changed={}", lua_dir.display());
    std::fs::write(
        out_dir.join("main.lua"),
        bundler::Bundler::new()
            .modules(lua_dir.join("lunest.lua"))?
            .public("lunest")
            .entry_point("lunest")
            .bundle(),
    )?;
    Ok(())
}
