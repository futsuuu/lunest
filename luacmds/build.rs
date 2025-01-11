use std::{env, path::PathBuf, sync::Arc};

const MIN_ZSTD_DICT_SAMPLES: usize = 7;

const FORCE_USING_ZSTD_DICT: bool = !cfg!(debug_assertions);
const MAX_ZSTD_DICT_SIZE: usize = 512 * 1024;
const ZSTD_COMPRESSION_LEVEL: i32 = if cfg!(debug_assertions) { 3 } else { 22 };

fn main() -> std::io::Result<()> {
    let versions = ["lua54", "lua53", "lua52", "lua51", "luajit"];

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let opt_level = env::var("OPT_LEVEL").unwrap();

    println!("cargo::rerun-if-changed=../build.zig");
    println!("cargo::rerun-if-changed=../build.zig.zon");
    println!("cargo::rerun-if-changed=../lua-rt");

    {
        let mut c = std::process::Command::new("zig");
        c.args(["build", "-j1"]);
        match opt_level.as_str() {
            "s" | "z" => {
                c.arg("--release=small");
            }
            "1" => {
                c.arg("--release=safe");
            }
            "2" | "3" => {
                c.arg("--release=fast");
            }
            _ => (),
        }
        assert!(c.status()?.success());
    }

    let mut artifacts = Vec::new();
    let bin_dir = PathBuf::from("../zig-out/bin");
    for version in versions {
        let bin_name = format!("{version}{}", env::consts::EXE_SUFFIX);
        let contents = std::fs::read(bin_dir.join(bin_name))?;
        artifacts.push((version, contents));
    }

    println!(r#"cargo::rustc-check-cfg=cfg(zstd_dict)"#);

    if artifacts.is_empty() {
        return Ok(());
    }

    let dict = if MIN_ZSTD_DICT_SAMPLES <= artifacts.len() || FORCE_USING_ZSTD_DICT {
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
    };

    let dict = Arc::new(dict);
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
