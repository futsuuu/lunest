const ZSTD_COMPRESSION_LEVEL: i32 = if cfg!(debug_assertions) { 3 } else { 22 };

fn main() -> std::io::Result<()> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let versions = ["lua54", "lua53", "lua52", "lua51"];

    println!("cargo::rerun-if-changed=../build.zig");
    println!("cargo::rerun-if-changed=../build.zig.zon");
    println!("cargo::rerun-if-changed=./main.zig");

    let mut c = std::process::Command::new("zig");
    c.arg("build");
    let target = {
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
        let mut abi = std::env::var("CARGO_CFG_TARGET_ENV").unwrap();
        abi.push_str(&std::env::var("CARGO_CFG_TARGET_ABI").unwrap());
        if abi.is_empty() {
            abi = "none".into();
        }
        format!("-Dtarget={arch}-{os}-{abi}")
    };
    eprintln!("zig target: {target}");
    c.arg(target);
    let optimize = match std::env::var("OPT_LEVEL").unwrap().as_str() {
        "s" | "z" => "-Doptimize=ReleaseSmall",
        "2" | "3" => "-Doptimize=ReleaseFast",
        "1" => "-Doptimize=ReleaseSafe",
        "0" => "-Doptimize=Debug",
        _ => unreachable!(),
    };
    eprintln!("zig optimize mode: {optimize}");
    c.arg(optimize);
    assert!(c.status()?.success());

    let mut artifacts = Vec::new();
    let bin_dir = std::path::PathBuf::from("../zig-out/bin");
    for version in versions {
        let bin_name = format!("{version}{}", std::env::consts::EXE_SUFFIX);
        let contents = std::fs::read(bin_dir.join(bin_name))?;
        artifacts.push((version, contents));
    }

    let mut threads = Vec::new();
    for (version, contents) in artifacts {
        std::fs::write(
            out_dir.join(format!("{version}_size.rs")),
            format!("{}", contents.len()),
        )?;
        let out = out_dir.join(format!("{version}.zst"));
        threads.push(std::thread::spawn(move || -> std::io::Result<()> {
            eprintln!("compressing {version}...");
            let writer = std::fs::File::create(out)?;
            let mut encoder = zstd::Encoder::new(writer, ZSTD_COMPRESSION_LEVEL)?;
            encoder.include_dictid(false)?;
            encoder.include_checksum(false)?;
            let mut encoder = encoder.auto_finish();
            let mut contents = contents.as_slice();
            std::io::copy(&mut contents, &mut encoder)?;
            eprintln!("done: {version}");
            Ok(())
        }));
    }
    for handle in threads {
        handle.join().expect("failed to join thread")?;
    }

    Ok(())
}
