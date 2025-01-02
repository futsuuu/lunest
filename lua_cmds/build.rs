use std::{env, path::PathBuf};

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let cargo_exe = env::var_os("CARGO").unwrap();
    let build_profile = env::var("PROFILE").unwrap();
    let target_triple = env::var("TARGET").unwrap();

    let lua_cmd_dir = PathBuf::from("../lua_cmd");
    assert!(lua_cmd_dir.exists());
    println!("cargo::rerun-if-changed={}", lua_cmd_dir.display());
    let bin_name = format!("lua_cmd{}", env::consts::EXE_SUFFIX);

    let target_dir = out_dir.join("target");

    for version in [
        #[cfg(feature = "lua51")]
        "lua51",
        #[cfg(feature = "lua52")]
        "lua52",
        #[cfg(feature = "lua53")]
        "lua53",
        #[cfg(feature = "lua54")]
        "lua54",
        #[cfg(feature = "luajit")]
        "luajit",
    ] {
        let mut c = std::process::Command::new(&cargo_exe);
        c.arg("build");
        c.arg("--manifest-path").arg(lua_cmd_dir.join("Cargo.toml"));
        c.args(["--features", version, "--no-default-features"]);
        c.args(["--target", target_triple.as_str()]);
        match build_profile.as_str() {
            "debug" => (),
            "release" => {
                c.arg("--release");
            }
            profile => {
                c.args(["--profile", profile]);
            }
        }

        c.arg("--target-dir").arg(&target_dir);
        c.args(["--color", "always"]);

        assert!(c.status()?.success());
        std::fs::copy(
            target_dir
                .join(&target_triple)
                .join(&build_profile)
                .join(&bin_name),
            out_dir.join(version),
        )?;
    }
    Ok(())
}
