use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(
        out_dir.join("main.lua"),
        bundle(
            "lunest",
            &env::current_dir()?.join("lua").join("lunest.lua"),
        )?,
    )?;
    Ok(())
}

fn bundle(module_name: &str, entry_point: &Path) -> Result<String> {
    let entrypoint = fs::canonicalize(entry_point)?;
    let module_dir = if entrypoint.file_name().unwrap() == OsStr::new("init.lua") {
        entrypoint.parent().unwrap().to_path_buf()
    } else {
        entrypoint.with_extension("")
    };

    let mut paths = Vec::new();
    paths.push(entrypoint.to_path_buf());
    for entry in walkdir::WalkDir::new(&module_dir) {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type().is_file()
            && path.extension() == Some(OsStr::new("lua"))
            && path != entrypoint
        {
            paths.push(entry.into_path());
        }
    }

    let mut result = String::from(
        "local function set_loaded(loaded, modname, mod)
    if loaded[modname] then
        return loaded[modname]
    elseif mod == nil then
        loaded[modname] = true
    else
        loaded[modname] = mod
    end
    return mod
end
local preload = {}
local loaded = {}
local require = function(modname)
    if loaded[modname] ~= nil then
        return loaded[modname]
    elseif preload[modname] then
        return set_loaded(loaded, modname, preload[modname](modname))
    else
        return require(modname)
    end
end
",
    );
    let base_dir = module_dir.parent().unwrap();
    for path in &paths {
        let contents = fs::read_to_string(path)?;
        let path = path.strip_prefix(base_dir).unwrap();
        let modname = if path.file_name().unwrap() == OsStr::new("init.lua") {
            path.parent().unwrap().display().to_string()
        } else {
            path.with_extension("").display().to_string()
        }
        .replace(['/', '\\'], ".");
        result += &format!("preload['{modname}'] = function(...)\n{contents}\nend\n");
        println!("cargo:rerun-if-changed={}", path.display());
    }

    result += &format!(
        "return set_loaded(package.loaded, '{}', require('{}'))\n",
        module_name,
        module_dir.file_name().unwrap().to_str().unwrap()
    );

    Ok(result)
}
