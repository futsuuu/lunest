use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result};
use merge::Merge;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    profile: HashMap<String, Profile>,
}

#[derive(Clone, Debug, Deserialize, Merge)]
pub struct Profile {
    lua: Option<Vec<String>>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    init: Option<PathBuf>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            lua: Some(vec!["lua".into()]),
            include: Some(vec!["{src,lua}/**/*.lua".into()]),
            exclude: Some(vec![]),
            init: None,
        }
    }
}

impl Config {
    pub fn read(root_dir: &Path) -> Result<Config> {
        let paths = [
            root_dir.join(".config").join("lunest.toml"),
            root_dir.join("lunest.toml"),
            root_dir.join(".lunest.toml"),
        ];
        let config = if let Some(s) = paths.iter().find_map(|p| fs::read_to_string(p).ok()) {
            toml::from_str(&s)?
        } else {
            Self::default()
        };
        Ok(config)
    }

    pub fn profile<'a>(&'a self, name: Option<&'a str>) -> Result<(&'a str, Profile)> {
        let (name, mut profile) = if let Some(name) = name {
            let mut profile = self
                .profile
                .get(name)
                .with_context(|| format!("profile '{name}' is not defined"))?
                .clone();
            if let Some(default) = self.profile.get("default") {
                profile.merge(default.clone());
            }
            (name, profile)
        } else if self.profile.is_empty() {
            return Ok(("default", Profile::default()));
        } else if self.profile.keys().nth(1).is_none() {
            let (name, profile) = self.profile.iter().next().unwrap();
            (name.as_str(), profile.clone())
        } else if let Some(default) = self.profile.get("default") {
            ("default", default.clone())
        } else {
            anyhow::bail!("you must specify the profile or define a 'default' profile");
        };

        if let (Some(exclude), Some(path)) = (profile.exclude.as_mut(), profile.init.as_ref()) {
            exclude.push(path.display().to_string());
        }
        profile.merge(Profile::default());

        Ok((name, profile))
    }
}

impl Profile {
    pub fn lua_command(&self) -> Result<process::Command> {
        let lua = self.lua.as_ref().unwrap();
        let mut cmd = process::Command::new(lua.first().context("command is empty")?);
        cmd.args(lua.get(1..).unwrap_or_default());
        Ok(cmd)
    }

    pub fn target_files(&self, root_dir: &Path) -> Result<Vec<PathBuf>> {
        let include = build_globset(self.include.as_ref().unwrap())?;
        let exclude = build_globset(self.exclude.as_ref().unwrap())?;
        let mut r = Vec::new();
        for entry in walkdir::WalkDir::new(root_dir)
            .follow_links(true)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|entry| {
                let path = entry.path().strip_prefix(root_dir).unwrap();
                if exclude.is_match(path) {
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
                r.push(entry.into_path())
            }
        }
        Ok(r)
    }

    pub fn init_file(&self) -> Result<Option<&Path>> {
        if let Some(path) = self.init.as_ref() {
            anyhow::ensure!(
                path.exists(),
                "init file `{}` does not exist",
                path.display(),
            );
        }
        Ok(self.init.as_deref())
    }
}

fn build_globset(patterns: &[String]) -> Result<globset::GlobSet> {
    let mut builder = globset::GlobSet::builder();
    for pat in patterns {
        builder.add(globset::Glob::new(pat)?);
    }
    Ok(builder.build()?)
}
