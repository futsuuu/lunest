use std::{collections::HashMap, env::current_dir, fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    profile: HashMap<String, Profile>,
}

#[derive(Deserialize, Clone)]
pub struct Profile {
    lua: Option<Vec<String>>,
    setup: Option<String>,
    files: Option<Vec<String>>,
}

impl Config {
    pub fn get_profile(profile: &str) -> Result<Profile> {
        let file_path = current_dir()?.join(".lunest").join("config.toml");
        let config: Config = toml::from_str(&fs::read_to_string(file_path)?)?;

        let default = config.profile.get("default").cloned().unwrap_or_default();
        let profile = config
            .profile
            .get(profile)
            .with_context(|| format!("cannot get profile '{profile}'"))?;
        Ok(Profile {
            lua: or(&profile.lua, &default.lua),
            setup: or(&profile.setup, &default.setup),
            files: or(&profile.files, &default.files),
        })
    }
}

fn or<T: Clone>(a: &Option<T>, b: &Option<T>) -> Option<T> {
    a.as_ref().or(b.as_ref()).cloned()
}

impl Profile {
    pub fn get_lua(&self) -> Result<&Vec<String>> {
        self.lua.as_ref().context("field 'lua' not specified")
    }

    pub fn get_setup(&self) -> Result<&Path> {
        self.setup
            .as_ref()
            .context("field 'setup' not specified")
            .map(Path::new)
    }

    pub fn get_files(&self) -> Result<&Vec<String>> {
        self.files.as_ref().context("field 'files' not specified")
    }
}

impl Default for Profile {
    fn default() -> Profile {
        Profile {
            lua: Some(vec![String::from("lua")]),
            setup: None,
            files: Some(vec![String::from(r"**/*[\._]{test,spec}.lua")]),
        }
    }
}
