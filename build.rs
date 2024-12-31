fn main() -> std::io::Result<()> {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=./lua");
    std::fs::write(
        out_dir.join("main.lua"),
        bundler::Bundler::new()
            .modules("./lua/lunest.lua")?
            .modules("./lib/json.lua/json.lua")?
            .public("lunest")
            .entry_point("lunest")
            .bundle(),
    )?;
    Ok(())
}
