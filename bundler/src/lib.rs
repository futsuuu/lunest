use std::{ffi::OsStr, path::Path};

#[derive(Default)]
pub struct Bundler {
    modules: Vec<Module>,
    publics: Vec<String>,
}

impl Bundler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_modules(
        &mut self,
        module_root_file: impl AsRef<Path>,
    ) -> std::io::Result<&mut Self> {
        let module_root_file = module_root_file.as_ref();
        let module_dir = if module_root_file.file_name().unwrap() == OsStr::new("init.lua") {
            module_root_file.parent().unwrap().to_path_buf()
        } else {
            #[cfg(feature = "build-script")]
            println!("cargo::rerun-if-changed={}", module_root_file.display());
            module_root_file.with_extension("")
        };

        self.modules
            .push(Module::new(module_dir.parent().unwrap(), module_root_file)?);
        if module_dir.exists() {
            #[cfg(feature = "build-script")]
            println!("cargo::rerun-if-changed={}", module_dir.display());
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

    pub fn make_public(&mut self, module_name: impl Into<String>) -> &mut Self {
        self.publics.push(module_name.into());
        self
    }

    pub fn bundle(&self, default_module: Option<impl AsRef<str>>) -> String {
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
        if let Some(m) = default_module {
            result += &format!("return require('{}')\n", m.as_ref());
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
            chunk: match std::fs::read_to_string(path) {
                Err(e) => Err(std::io::Error::new(
                    e.kind(),
                    format!("failed to read `{}` to string: {e}", path.display()),
                )),
                ok => ok,
            }?,
        })
    }

    fn setup_loader(&self) -> String {
        let modname = &self.name;
        let chunk = self.chunk.trim();
        format!("package.preload['{modname}'] = function(...)\n{chunk}\nend\n")
    }
}
