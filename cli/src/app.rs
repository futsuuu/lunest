use anyhow::Context as _;
use merge::Merge as _;

pub struct App {
    root_dir: std::path::PathBuf,
    profiles: Vec<crate::profile::Profile>,

    temp_dir: tempfile::TempDir,
    main_script: std::path::PathBuf,
    program_cache:
        std::cell::RefCell<std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>>,
    process_dir_counter: std::cell::Cell<usize>,
}

#[derive(clap::Args, Debug)]
pub struct Options {
    /// Load Lua files with the specified profile
    #[arg(long, short, value_delimiter = ',')]
    profile: Vec<String>,
    /// Load Lua files with the profiles in the specified group
    #[arg(long, short, value_delimiter = ',')]
    group: Vec<String>,
    /// Don't clean up a temporary directory on exit
    #[arg(long)]
    keep_tmpdir: bool,
}

impl App {
    pub fn new(opts: Options) -> anyhow::Result<Self> {
        log::trace!("creating new app context");

        let (root_dir, config_file) = find_config_file(std::env::current_dir()?);
        let spec: Specifier = if let Some(path) = config_file {
            toml::from_str(&std::fs::read_to_string(path)?)?
        } else {
            Specifier::default()
        };
        let profiles = spec.profiles(&root_dir, opts.profile, opts.group)?;
        assert!(!profiles.is_empty());

        let temp_dir = tempfile::Builder::new()
            .prefix(env!("CARGO_PKG_NAME"))
            .keep(opts.keep_tmpdir)
            .tempdir()?;
        let main_script = temp_dir.path().join("main.lua");
        std::fs::write(
            &main_script,
            include_str!(concat!(env!("OUT_DIR"), "/main.lua")),
        )?;
        Ok(Self {
            root_dir,
            profiles,
            temp_dir,
            main_script,
            program_cache: std::cell::RefCell::new(std::collections::HashMap::new()),
            process_dir_counter: std::cell::Cell::new(0),
        })
    }

    pub fn root_dir(&self) -> &std::path::Path {
        &self.root_dir
    }

    pub fn profiles(&self) -> &[crate::profile::Profile] {
        &self.profiles
    }

    pub fn create_process_dir(&self) -> std::io::Result<std::path::PathBuf> {
        let counter = self.process_dir_counter.get();
        let name = format!("p{:x}", counter);
        self.process_dir_counter.set(counter + 1);
        let dir = self.temp_dir.path().join(name);
        std::fs::create_dir(&dir)?;
        Ok(dir)
    }

    pub fn get_main_script(&self) -> &std::path::Path {
        &self.main_script
    }

    pub fn get_lua_program(
        &self,
        name: impl AsRef<std::ffi::OsStr>,
    ) -> std::io::Result<std::ffi::OsString> {
        let name = name.as_ref();
        if let Some(program) = self.program_cache.borrow().get(name) {
            return Ok(program.clone());
        }
        let program: std::ffi::OsString = if let Ok(path) = which::which(name) {
            path.into()
        } else if let Some(lua) = lua_rt::Lua::from_program_name(name) {
            let path = self.temp_dir.path().join(lua.recommended_program_name());
            lua.write(&path)?;
            path.into()
        } else {
            name.into()
        };
        self.program_cache
            .borrow_mut()
            .insert(name.to_os_string(), program.clone());
        Ok(program)
    }
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct Specifier {
    group: std::collections::HashMap<String, Vec<String>>,
    profile: std::collections::HashMap<String, crate::profile::Specifier>,
}

impl Specifier {
    fn profiles(
        mut self,
        root_dir: &std::path::Path,
        mut profile_names: Vec<String>,
        group_names: Vec<String>,
    ) -> anyhow::Result<Vec<crate::profile::Profile>> {
        for group_name in group_names {
            let group = self
                .group
                .get(&group_name)
                .with_context(|| format!("'{group_name}' group is not defined"))?;
            for profile_name in group {
                anyhow::ensure!(
                    self.profile.contains_key(profile_name),
                    "'{profile_name}' profile specified in '{group_name}' group is not defined",
                );
                profile_names.push(profile_name.clone());
            }
        }
        profile_names.dedup();
        if profile_names.is_empty() {
            if self.profile.contains_key("default") {
                profile_names.push("default".to_string());
            } else if self.profile.len() == 1 {
                profile_names.push(self.profile.keys().next().unwrap().to_string());
            } else {
                anyhow::bail!("you must specify the profile or define a 'default' profile");
            }
        }

        let default_profile_spec = if let Some(mut p) = self.profile.get("default").cloned() {
            p.merge(crate::profile::Specifier::default());
            p
        } else {
            crate::profile::Specifier::default()
        };
        let mut profiles = Vec::new();
        for profile_name in profile_names {
            let mut profile_spec = self
                .profile
                .remove(&profile_name)
                .with_context(|| format!("'{profile_name}' profile is not defined"))?;
            profile_spec.merge(default_profile_spec.clone());
            profiles.push(crate::profile::Profile::from_spec(
                profile_name,
                profile_spec,
                root_dir,
            )?);
        }
        Ok(profiles)
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
