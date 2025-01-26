use anyhow::Context;
use merge::Merge;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct ConfigSpec {
    group: std::collections::HashMap<String, Vec<String>>,
    profile: std::collections::HashMap<String, ProfileSpec>,
}

#[derive(Clone, Debug, serde::Deserialize, Merge)]
struct ProfileSpec {
    lua: Option<Vec<String>>,
    include: Option<globset::GlobSet>,
    exclude: Option<globset::GlobSet>,
    init: Option<std::path::PathBuf>,
}

impl Default for ProfileSpec {
    fn default() -> Self {
        static DEFAULT: std::sync::OnceLock<ProfileSpec> = std::sync::OnceLock::new();
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

pub struct Config {
    root_dir: std::path::PathBuf,
    profiles: std::collections::HashMap<String, std::rc::Rc<Profile>>,
    groups: std::collections::HashMap<String, Vec<std::rc::Rc<Profile>>>,
}

impl Config {
    pub fn read() -> anyhow::Result<Self> {
        log::trace!("reading configuration");
        let (root_dir, config_file) = find_config_file(std::env::current_dir()?);
        if let Some(path) = config_file {
            Self::from_spec(toml::from_str(&std::fs::read_to_string(path)?)?, root_dir)
        } else {
            Ok(Self {
                root_dir,
                profiles: std::collections::HashMap::new(),
                groups: std::collections::HashMap::new(),
            })
        }
    }

    pub fn root_dir(&self) -> &std::path::Path {
        &self.root_dir
    }

    pub fn profile(&self, name: &str) -> anyhow::Result<&Profile> {
        let profile = self
            .profiles
            .get(name)
            .with_context(|| format!("profile '{name}' is not defined"))?;
        Ok(profile)
    }

    pub fn default_profile(&self) -> anyhow::Result<&Profile> {
        if let Some(profile) = self.profiles.get("default") {
            return Ok(profile);
        } else if self.profiles.len() == 1 {
            return Ok(self.profiles.values().next().unwrap());
        }
        anyhow::bail!("you must specify the profile or define a 'default' profile");
    }

    pub fn group(&self, name: &str) -> anyhow::Result<impl Iterator<Item = &Profile>> {
        let group = self
            .groups
            .get(name)
            .with_context(|| format!("group '{name}' is not defined"))?;
        Ok(group.iter().map(std::rc::Rc::as_ref))
    }

    fn from_spec(config_spec: ConfigSpec, root_dir: std::path::PathBuf) -> anyhow::Result<Self> {
        let profiles = {
            let mut profiles = std::collections::HashMap::new();
            let default_spec = if let Some(mut spec) = config_spec.profile.get("default").cloned() {
                spec.merge(ProfileSpec::default());
                spec
            } else {
                ProfileSpec::default()
            };
            for (name, mut spec) in config_spec.profile {
                spec.merge(default_spec.clone());
                let profile = Profile::from_spec(name.clone(), spec, &root_dir)?;
                profiles.insert(name, profile.into());
            }
            profiles
        };
        let groups = {
            let mut groups = std::collections::HashMap::new();
            for (name, group_spec) in config_spec.group {
                let mut group = Vec::new();
                for member in group_spec {
                    let profile = profiles
                        .get(&member)
                        .with_context(|| format!("profile '{member}' is not defined"))?;
                    group.push(std::rc::Rc::clone(profile));
                }
                groups.insert(name, group);
            }
            groups
        };
        Ok(Self {
            root_dir,
            profiles,
            groups,
        })
    }
}

fn find_config_file(cwd: std::path::PathBuf) -> (std::path::PathBuf, Option<std::path::PathBuf>) {
    log::trace!("finding configuration file");
    let mut dir = cwd.as_path();
    loop {
        if let Some(config) = [dir.join("lunest.toml"), dir.join(".lunest.toml")]
            .into_iter()
            .find(|p| p.exists())
        {
            log::info!("configuration file found");
            log::debug!("root directory: {dir:?}");
            log::debug!("configuration file: {config:?}");
            break (dir.to_path_buf(), Some(config));
        }
        let Some(parent) = dir.parent() else {
            log::info!("configuration file not found");
            break (cwd, None);
        };
        dir = parent;
    }
}

#[cfg(test)]
mod find_config_file_tests {
    use super::*;

    #[test]
    fn found() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let cwd = temp_dir.path().join("a/b/c");
        std::fs::create_dir_all(&cwd)?;
        for s in ["a/lunest.toml", "a/b/lunest.toml", "a/b/.lunest.toml"] {
            std::fs::write(temp_dir.path().join(s), "")?;
        }
        assert_eq!(
            (
                temp_dir.path().join("a").join("b"),
                Some(temp_dir.path().join("a").join("b").join("lunest.toml"))
            ),
            find_config_file(cwd)
        );
        Ok(())
    }

    #[test]
    fn not_found() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let cwd = temp_dir.path().join("a/b/c");
        std::fs::create_dir_all(&cwd)?;
        assert_eq!((cwd.clone(), None), find_config_file(cwd));
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct Profile {
    name: String,
    init_script: Option<std::path::PathBuf>,
    target_files: Vec<std::path::PathBuf>,
    lua_program: String,
    lua_args: Vec<String>,
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

    pub fn lua_command(
        &self,
        cx: &crate::global::Context,
    ) -> std::io::Result<std::process::Command> {
        let mut c = std::process::Command::new(&*cx.get_lua_program(&self.lua_program)?);
        c.args(&self.lua_args);
        Ok(c)
    }

    fn from_spec(
        name: String,
        spec: ProfileSpec,
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
        let lua = spec.lua.as_ref().unwrap();
        Ok(Self {
            name,
            init_script,
            target_files,
            lua_program: lua.first().context("'lua' field is empty")?.to_string(),
            lua_args: lua.get(1..).unwrap_or_default().to_vec(),
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

#[cfg(test)]
mod profile_tests {
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
        let spec = ProfileSpec {
            lua: Some(vec!["foo".into(), "hello world".into(), "!".into()]),
            ..Default::default()
        };
        let p = Profile::from_spec("name".into(), spec, root_dir.path()).unwrap();
        assert_eq!(String::from("foo"), p.lua_program);
        assert_eq!(
            vec![String::from("hello world"), String::from("!")],
            p.lua_args
        );
    }

    #[rstest]
    fn lua_command_error(root_dir: tempfile::TempDir) {
        let spec = ProfileSpec {
            lua: Some(Vec::new()),
            ..Default::default()
        };
        assert!(Profile::from_spec("name".into(), spec, root_dir.path()).is_err());
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
