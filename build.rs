use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    bundle(&out_dir)?;
    Ok(())
}

fn bundle(out_dir: &Path) -> Result<()> {
    let mut result = String::from(
        "local preload = {}
local require = function(modname)
    if package.loaded[modname] then
        return package.loaded[modname]
    end
    if preload[modname] then
        local ret = preload[modname](modname)
        if package.loaded[modname] then
            return package.loaded[modname]
        end
        if ret == nil then
            package.loaded[modname] = true
        else
            package.loaded[modname] = ret
        end
        return ret
    else
        return require(modname)
    end
end
",
    );
    let current_dir = env::current_dir()?;
    let lua_dir = current_dir.join("lua");
    println!("cargo:rerun-if-changed={}", lua_dir.display());

    for entry in walkdir::WalkDir::new(&lua_dir) {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension() != Some(OsStr::new("lua")) {
            continue;
        }

        let contents = fs::read_to_string(path)?;
        let modname = {
            let path = path.strip_prefix(&lua_dir).unwrap();
            if path.file_name() == Some(OsStr::new("init.lua")) {
                path.parent().unwrap().display().to_string()
            } else {
                path.with_extension("").display().to_string()
            }
            .replace(['/', '\\'], ".")
        };
        result += &format!("preload['{modname}'] = function(...)\n{contents}\nend\n");
    }

    result += "require('lunest')\n";

    fs::write(out_dir.join("main.lua"), result)?;

    Ok(())
}
