fn main() -> std::io::Result<()> {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(
        out_dir.join("main.lua"),
        bundler::Bundler::new()
            .add_modules("../module/lunest.lua")?
            .add_modules("../3rd/json.lua/json.lua")?
            .make_public("lunest")
            .bundle(Some("lunest")),
    )
}
