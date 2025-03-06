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
            &GlobSet::new(
                spec.include.unwrap_or_default().as_slice(),
                spec.exclude.unwrap_or_default().as_slice(),
            )?,
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

#[derive(Debug)]
struct GlobSet {
    included_dirs: globset::GlobSet,
    included_files: globset::GlobSet,
    excluded_files: globset::GlobSet,
}

impl GlobSet {
    fn new(include: &[String], exclude: &[String]) -> Result<Self, globset::Error> {
        let mut included_dirs = globset::GlobSetBuilder::new();
        let mut included_files = globset::GlobSetBuilder::new();
        let mut pattern_set = std::collections::HashSet::new();
        for file_pattern in include {
            if !pattern_set.contains(file_pattern) {
                included_files.add(new_glob(file_pattern)?);
                pattern_set.insert(file_pattern.to_string());
            }
            for slash_index in file_pattern
                .char_indices()
                .filter_map(|(i, c)| (c == '/').then_some(i))
            {
                let dir_pattern = file_pattern.get(..slash_index).unwrap();
                if !pattern_set.contains(dir_pattern) {
                    included_dirs.add(new_glob(dir_pattern)?);
                    pattern_set.insert(dir_pattern.to_string());
                }
            }
        }
        let mut excluded_files = globset::GlobSetBuilder::new();
        for file_pattern in exclude {
            excluded_files.add(new_glob(file_pattern)?);
        }
        Ok(Self {
            included_dirs: included_dirs.build()?,
            included_files: included_files.build()?,
            excluded_files: excluded_files.build()?,
        })
    }

    fn is_match(&self, relative_path: &std::path::Path, is_directory: bool) -> bool {
        if is_directory {
            self.included_dirs.is_match(relative_path)
        } else {
            self.included_files.is_match(relative_path)
                && !self.excluded_files.is_match(relative_path)
        }
    }
}

fn new_glob(glob: &str) -> Result<globset::Glob, globset::Error> {
    globset::GlobBuilder::new(glob)
        .empty_alternates(true)
        .literal_separator(true)
        .build()
}

fn is_entry_valid(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_file() || entry.file_type().is_dir()
}

fn is_entry_init_script(
    entry: &walkdir::DirEntry,
    init_script: Option<&std::path::PathBuf>,
) -> bool {
    init_script.is_some_and(|p| p == entry.path() && entry.file_type().is_file())
}

#[cfg(test)]
mod globset_tests {
    use super::*;

    #[test]
    fn match_file_without_excluding() {
        let p = GlobSet::new(&["**/*.lua".into()], &[]).unwrap();
        assert!(p.is_match(std::path::Path::new("a.lua"), false));
        assert!(p.is_match(std::path::Path::new("a/b.lua"), false));
        assert!(!p.is_match(std::path::Path::new("a.txt"), false));
    }

    #[test]
    fn match_file_with_excluding() {
        let p = GlobSet::new(&["**/*.lua".into()], &["a/*.lua".into()]).unwrap();
        assert!(p.is_match(std::path::Path::new("a.lua"), false));
        assert!(!p.is_match(std::path::Path::new("a/b.lua"), false));
        assert!(p.is_match(std::path::Path::new("a/b/c.lua"), false));
    }

    #[test]
    fn match_directory() {
        let p = GlobSet::new(&["a/**/*.lua".into()], &["a*/**/*.lua".into()]).unwrap();
        assert!(p.is_match(std::path::Path::new("a/b"), true));
        assert!(!p.is_match(std::path::Path::new("b/c"), true));
    }
}

fn target_files(
    root_dir: &std::path::Path,
    globset: &GlobSet,
    init_script: Option<&std::path::PathBuf>,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    log::trace!("reading target files");
    let mut r = Vec::new();
    for entry in walkdir::WalkDir::new(root_dir)
        .min_depth(1)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            if !is_entry_valid(entry) || is_entry_init_script(entry, init_script) {
                false
            } else {
                globset.is_match(
                    entry.path().strip_prefix(root_dir).unwrap(),
                    entry.file_type().is_dir(),
                )
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
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub init: Option<std::path::PathBuf>,
}

impl Default for Specifier {
    fn default() -> Self {
        Self {
            lua: Some(vec!["lua".into()]),
            include: Some(vec!["{src,lua}/**/*.lua".into()]),
            exclude: Some(vec![]),
            init: None,
        }
    }
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
                &GlobSet::new(
                    &["lua/**/*.lua".into(), "test/a*.lua".into()],
                    &["lua/hello.lua".into()]
                )?,
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
                &GlobSet::new(&["test/**/*.lua".into()], &[])?,
                Some(&root.join("test/abc.lua")),
            )?,
        );
        Ok(())
    }
}
