use std::{ffi::OsStr, fs, path::Path};

#[derive(Default)]
pub struct Bundler {
    modules: Vec<Module>,
    entry_point: Option<String>,
    publics: Vec<String>,
}

impl Bundler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn modules(&mut self, module_root_file: impl AsRef<Path>) -> std::io::Result<&mut Self> {
        let module_root_file = module_root_file.as_ref();
        let module_dir = if module_root_file.file_name().unwrap() == OsStr::new("init.lua") {
            module_root_file.parent().unwrap().to_path_buf()
        } else {
            module_root_file.with_extension("")
        };

        self.modules
            .push(Module::new(module_dir.parent().unwrap(), module_root_file)?);
        if module_dir.exists() {
            for entry in walkdir::WalkDir::new(&module_dir) {
                let entry = entry?;
                let path = entry.path();
                if entry.file_type().is_file()
                    && path.extension() == Some(OsStr::new("lua"))
                    && path != module_root_file
                {
                    self.modules
                        .push(Module::new(module_dir.parent().unwrap(), entry.path())?);
                }
            }
        }

        Ok(self)
    }

    pub fn public(&mut self, module_name: &str) -> &mut Self {
        self.publics.push(module_name.into());
        self
    }

    pub fn entry_point(&mut self, module_name: &str) -> &mut Self {
        self.entry_point = Some(module_name.into());
        self
    }

    pub fn bundle(&self) -> String {
        let mut result = include_str!("./override.lua").replace(
            "local PUBLIC_MODULES\n",
            &format!(
                "local PUBLIC_MODULES = {{ {} }}\n",
                self.publics
                    .iter()
                    .map(|m| format!("['{m}'] = true"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        for module in &self.modules {
            result += &module.setup_loader();
        }
        if let Some(entry_point) = &self.entry_point {
            result += &format!("return require('{entry_point}')\n");
        }
        result
    }
}

struct Module {
    name: String,
    chunk: String,
}

impl Module {
    fn new(base_dir: &Path, path: &Path) -> std::io::Result<Self> {
        Ok(Self {
            name: {
                let path = path.strip_prefix(base_dir).unwrap();
                if path.file_name().unwrap() == OsStr::new("init.lua") {
                    path.parent().unwrap().display().to_string()
                } else {
                    path.with_extension("").display().to_string()
                }
                .replace(['/', '\\'], ".")
            },
            chunk: fs::read_to_string(path)?,
        })
    }

    fn setup_loader(&self) -> String {
        let modname = &self.name;
        let chunk = self.chunk.trim();
        format!("package.preload['{modname}'] = function(...)\n{chunk}\nend\n")
    }
}
