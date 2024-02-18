use std::{collections::HashMap, env::current_dir, fs, path::Path};

use anyhow::{Context, Result};
use merge::Merge;
use serde::Deserialize;

#[derive(Deserialize, Merge)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Config {
    profile: Option<HashMap<String, Profile>>,
}

#[derive(Deserialize, Merge, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Profile {
    #[serde(skip)]
    #[merge(skip)]
    name: String,
    lua: Option<Vec<String>>,
    setup: Option<String>,
    files: Option<Vec<String>>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let file_path = current_dir()?.join(".lunest").join("config.toml");
        if let Ok(config) = fs::read_to_string(file_path) {
            return Self::load_from(&config);
        }
        Ok(Self::default())
    }

    fn load_from(config: &str) -> Result<Self> {
        let mut config: Config = toml::from_str(&config)?;
        config.merge(Config::default());
        Ok(config)
    }

    pub fn get_profile(&self, profile: &str) -> Result<Profile> {
        let profile_str = profile;
        let mut profile = self
            .profile
            .as_ref()
            .unwrap()
            .get(profile)
            .with_context(|| format!("cannot get profile '{profile}'"))?
            .clone();
        profile.name = profile_str.to_string();
        if let Some(default) = self.profile.as_ref().unwrap().get("default").cloned() {
            profile.merge(default);
        }
        profile.merge(Profile::default());
        Ok(profile)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile: {
                let mut map = HashMap::new();
                map.insert(String::from("default"), Profile::default());
                Some(map)
            },
        }
    }
}

#[cfg(test)]
mod config {
    use super::*;

    #[test]
    fn override_default_profile() -> Result<()> {
        let config = Config::load_from(
            r#"[profile.default]
setup = "setup.lua"
[profile.ci]
setup = "ci.lua"
files = ["**/*.lua"]"#,
        )?;
        assert_eq!(
            Profile {
                name: "ci".to_owned(),
                lua: Some(vec!["lua".to_owned()]),
                setup: Some("ci.lua".to_owned()),
                files: Some(vec!["**/*.lua".to_owned()]),
            },
            config.get_profile("ci")?
        );
        Ok(())
    }

    #[test]
    fn empty_equals_default() -> Result<()> {
        assert_eq!(Config::default(), Config::load_from("")?);
        Ok(())
    }
}

impl Profile {
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_lua(&self) -> &Vec<String> {
        self.lua.as_ref().unwrap()
    }

    pub fn get_setup(&self) -> Option<&Path> {
        self.setup.as_ref().map(Path::new)
    }

    pub fn get_files(&self) -> &Vec<String> {
        self.files.as_ref().unwrap()
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: String::new(), // this value is not used
            lua: Some(vec![String::from("lua")]),
            setup: None,
            files: Some(vec![String::from(r"**/*[\._]{test,spec}.lua")]),
        }
    }
}
