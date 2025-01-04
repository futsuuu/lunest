use std::{env, path::PathBuf, sync::Arc};

const MIN_ZSTD_DICT_SAMPLES: usize = 7;

const FORCE_USING_ZSTD_DICT: bool = true;
const MAX_ZSTD_DICT_SIZE: usize = 512 * 1024;
const ZSTD_COMPRESSION_LEVEL: i32 = if cfg!(debug_assertions) { 3 } else { 22 };

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let cargo_exe = env::var_os("CARGO").unwrap();
    let build_profile = env::var("PROFILE").unwrap();
    let target_triple = env::var("TARGET").unwrap();

    let luacmd_dir = PathBuf::from("../luacmd");
    assert!(luacmd_dir.exists());
    println!("cargo::rerun-if-changed={}", luacmd_dir.display());
    let bin_name = format!("luacmd{}", env::consts::EXE_SUFFIX);

    let target_dir = out_dir.join("target");

    let mut default_version = None;
    let mut artifacts = Vec::new();

    for version in [
        #[cfg(feature = "lua54")]
        "lua54",
        #[cfg(feature = "lua53")]
        "lua53",
        #[cfg(feature = "lua52")]
        "lua52",
        #[cfg(feature = "lua51")]
        "lua51",
        #[cfg(feature = "luajit")]
        "luajit",
    ] {
        default_version = default_version.or(Some(version));

        let mut c = std::process::Command::new(&cargo_exe);
        c.arg("build");
        c.arg("--manifest-path").arg(luacmd_dir.join("Cargo.toml"));
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

        eprintln!("building {version}...");
        assert!(c.status()?.success());
        let bin_path = target_dir
            .join(&target_triple)
            .join(&build_profile)
            .join(&bin_name);
        let contents = std::fs::read(bin_path)?;
        artifacts.push((version, contents));
    }

    println!(
        r#"cargo::rustc-check-cfg=cfg(default_lua, values("none", "lua54", "lua53", "lua52", "lua51", "luajit"))"#
    );
    println!(
        r#"cargo::rustc-cfg=default_lua="{}""#,
        default_version.unwrap_or("none")
    );

    println!(r#"cargo::rustc-check-cfg=cfg(zstd_dict)"#);

    if artifacts.is_empty() {
        return Ok(());
    }

    let dict = Arc::new(if MIN_ZSTD_DICT_SAMPLES <= artifacts.len() || FORCE_USING_ZSTD_DICT {
        println!("cargo::rustc-cfg=zstd_dict");
        let dict = zstd::dict::from_samples(
            artifacts
                .iter()
                .map(|x| x.1.clone())
                .cycle()
                .take(MIN_ZSTD_DICT_SAMPLES.next_multiple_of(artifacts.len()))
                .collect::<Vec<_>>()
                .as_slice(),
            MAX_ZSTD_DICT_SIZE,
        )?;
        std::fs::write(out_dir.join("zstd_dict"), &dict)?;
        Some(zstd::dict::EncoderDictionary::copy(
            dict.as_slice(),
            ZSTD_COMPRESSION_LEVEL,
        ))
    } else {
        None
    });

    let mut threads = Vec::new();

    for (version, contents) in artifacts {
        std::fs::write(
            out_dir.join(format!("{version}_size.rs")),
            format!("{}", contents.len()),
        )?;
        let dict = Arc::clone(&dict);
        let out = out_dir.join(format!("{version}.zst"));
        threads.push(std::thread::spawn(move || -> std::io::Result<()> {
            eprintln!("compressing {version}...");
            let writer = std::fs::File::create(out)?;
            let mut encoder = if let Some(dict) = dict.as_ref() {
                zstd::Encoder::with_prepared_dictionary(writer, dict)?
            } else {
                zstd::Encoder::new(writer, ZSTD_COMPRESSION_LEVEL)?
            };
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
