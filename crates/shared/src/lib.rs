pub mod config;

use std::{env, path::PathBuf, process::Command};

pub fn dll_path(lua_feature: &str) -> PathBuf {
    let lua_id = lua_feature.strip_prefix("lua").unwrap();
    #[cfg(not(windows))]
    let ext = "so";
    #[cfg(windows)]
    let ext = "dll";
    project_root::get_project_root()
        .unwrap()
        .join(format!("lua/lunest_lib.{lua_id}.{ext}"))
}

pub fn command_to_string(cmd: &Command) -> String {
    let program = cmd.get_program().to_string_lossy().to_string();
    let program = if program.as_str() == env!("CARGO") {
        String::from("cargo")
    } else {
        program
    };
    format!(
        "{program} {}",
        cmd.get_args()
            .map(|s| s.to_string_lossy().escape_debug().to_string())
            .map(|s| if s.contains(' ') {
                format!("\"{s}\"")
            } else {
                s
            })
            .collect::<Vec<String>>()
            .join(" ")
    )
}
