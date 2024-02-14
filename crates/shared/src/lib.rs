use std::{env, path::PathBuf, process::Command};

pub fn dll_path() -> PathBuf {
    let path = project_root::get_project_root().unwrap().join("lua");
    #[cfg(windows)]
    return path.join("lunest_lib.dll");
    #[cfg(not(windows))]
    return path.join("lunest_lib.so");
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
