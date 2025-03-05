use anyhow::Context as _;

#[derive(Debug, PartialEq)]
pub struct Profile {
    name: String,
    init_script: Option<std::path::PathBuf>,
    target_files: Vec<std::path::PathBuf>,
    lua_command: crate::command::Builder,
}

impl Profile {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn init_script(&self) -> &Option<std::path::PathBuf> {
        &self.init_script
    }

    pub fn target_files(&self) -> &[std::path::PathBuf] {
        &self.target_files
    }

    pub fn lua_command(&self) -> &crate::command::Builder {
        &self.lua_command
    }

    pub fn from_spec(
        name: String,
        spec: crate::profile::Specifier,
        root_dir: &std::path::Path,
    ) -> anyhow::Result<Self> {
        let init_script = spec.init.map(|mut path| {
            if path.is_relative() {
                path = root_dir.join(path);
            }
            std::fs::canonicalize(&path).unwrap_or(path)
        });
        let target_files = target_files(
            root_dir,
            &spec.include.unwrap_or_default(),
            &spec.exclude.unwrap_or_default(),
            init_script.as_ref(),
        )?;
        Ok(Self {
            name,
            init_script,
            target_files,
            lua_command: {
                let lua = spec.lua.as_ref().unwrap();
                let mut cmd = crate::command::Builder::new(
                    lua.first().context("'lua' field must not be empty")?,
                );
                cmd.args(lua.get(1..).unwrap_or_default());
                cmd
            },
        })
    }
}

fn target_files(
    root_dir: &std::path::Path,
    include: &globset::GlobSet,
    exclude: &globset::GlobSet,
    init_script: Option<&std::path::PathBuf>,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    log::trace!("reading target files");
    let mut r = Vec::new();
    for entry in walkdir::WalkDir::new(root_dir)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            let path = entry.path().strip_prefix(root_dir).unwrap();
            if exclude.is_match(path) || init_script.is_some_and(|init| init == path) {
                false
            } else if entry.file_type().is_dir() {
                true
            } else {
                include.is_match(path)
            }
        })
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            r.push(entry.into_path());
        }
    }
    log::debug!("{} files found", r.len());
    Ok(r)
}

#[derive(Clone, Debug, serde::Deserialize, merge::Merge)]
pub struct Specifier {
    pub lua: Option<Vec<String>>,
    pub include: Option<globset::GlobSet>,
    pub exclude: Option<globset::GlobSet>,
    pub init: Option<std::path::PathBuf>,
}

impl Default for Specifier {
    fn default() -> Self {
        static DEFAULT: std::sync::OnceLock<Specifier> = std::sync::OnceLock::new();
        let default = DEFAULT.get_or_init(|| Self {
            lua: Some(vec!["lua".into()]),
            include: Some(build_globset(&["{src,lua}/**/*.lua"]).unwrap()),
            exclude: Some(globset::GlobSet::empty()),
            init: None,
        });
        default.clone()
    }
}

fn build_globset(patterns: &[&str]) -> Result<globset::GlobSet, globset::Error> {
    let mut builder = globset::GlobSetBuilder::new();
    for glob in patterns {
        builder.add(globset::Glob::new(glob)?);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::{fixture, rstest};

    #[fixture]
    fn root_dir() -> tempfile::TempDir {
        let temp_dir = tempfile::tempdir().unwrap();
        for s in ["lua", "test", "foo"] {
            std::fs::create_dir_all(temp_dir.path().join(s)).unwrap();
        }
        for s in [
            "lua/hello.lua",
            "lua/world.lua",
            "foo/a.lua",
            "test/abc.lua",
            "test/bcd.lua",
        ] {
            std::fs::write(temp_dir.path().join(s), "").unwrap();
        }
        temp_dir
    }

    #[rstest]
    fn lua_command_ok(root_dir: tempfile::TempDir) {
        let spec = crate::profile::Specifier {
            lua: Some(vec!["foo".into(), "hello world".into(), "!".into()]),
            ..Default::default()
        };
        let p = crate::profile::Profile::from_spec("name".into(), spec, root_dir.path()).unwrap();
        assert_eq!(
            crate::command::Builder::new("foo")
                .args(["hello world", "!"])
                .clone(),
            p.lua_command
        );
    }

    #[rstest]
    fn lua_command_error(root_dir: tempfile::TempDir) {
        let spec = crate::profile::Specifier {
            lua: Some(Vec::new()),
            ..Default::default()
        };
        assert!(crate::profile::Profile::from_spec("name".into(), spec, root_dir.path()).is_err());
    }

    #[rstest]
    fn include_and_exclude(root_dir: tempfile::TempDir) -> anyhow::Result<()> {
        let root = root_dir.path();
        assert_eq!(
            vec![
                root.join("lua").join("world.lua"),
                root.join("test").join("abc.lua"),
            ],
            target_files(
                root,
                &build_globset(&["lua/**/*.lua", "test/a*.lua"])?,
                &build_globset(&["lua/hello.lua"])?,
                None,
            )?,
        );
        Ok(())
    }

    #[rstest]
    fn exclude_init_script(root_dir: tempfile::TempDir) -> anyhow::Result<()> {
        let root = root_dir.path();
        assert_eq!(
            vec![root.join("test").join("bcd.lua")],
            target_files(
                root,
                &build_globset(&["test/**/*.lua"])?,
                &build_globset(&[])?,
                Some(&std::path::PathBuf::from("test/abc.lua")),
            )?,
        );
        Ok(())
    }
}
