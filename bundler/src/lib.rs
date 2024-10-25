use std::{ffi::OsStr, fs, path::Path};

use anyhow::Result;

pub fn bundle(entry_point: &Path) -> Result<String> {
    let entry_point = fs::canonicalize(entry_point)?;
    let module_dir = if entry_point.file_name().unwrap() == OsStr::new("init.lua") {
        entry_point.parent().unwrap().to_path_buf()
    } else {
        entry_point.with_extension("")
    };

    let mut modules = Vec::new();
    modules.push(Module::new(
        module_dir.parent().unwrap(),
        &entry_point,
        true,
    )?);
    for entry in walkdir::WalkDir::new(&module_dir) {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type().is_file()
            && path.extension() == Some(OsStr::new("lua"))
            && path != entry_point
        {
            modules.push(Module::new(
                module_dir.parent().unwrap(),
                entry.path(),
                false,
            )?);
        }
    }

    let mut result = include_str!("./override.lua").replace(
        "local PUBLIC_MODULES\n",
        &format!(
            "local PUBLIC_MODULES = {{ {} }}\n",
            modules
                .iter()
                .filter(|m| m.public)
                .map(|m| format!("['{}'] = true", m.name))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );

    for module in &modules {
        result += &module.preload();
    }

    result += &format!(
        "return require('{}')\n",
        module_dir.file_name().unwrap().to_str().unwrap()
    );

    Ok(result)
}

struct Module {
    name: String,
    contents: String,
    public: bool,
}

impl Module {
    fn new(root_dir: &Path, path: &Path, public: bool) -> Result<Self> {
        let contents = fs::read_to_string(path)?.trim().to_string();
        let path = path.strip_prefix(root_dir).unwrap();
        let name = if path.file_name().unwrap() == OsStr::new("init.lua") {
            path.parent().unwrap().display().to_string()
        } else {
            path.with_extension("").display().to_string()
        }
        .replace(['/', '\\'], ".");
        Ok(Self {
            name,
            contents,
            public,
        })
    }

    fn preload(&self) -> String {
        let modname = &self.name;
        let contents = &self.contents;
        format!("package.preload['{modname}'] = function(...)\n{contents}\nend\n")
    }
}
