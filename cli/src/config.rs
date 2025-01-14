use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use merge::Merge;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    group: std::collections::HashMap<String, Vec<String>>,
    profile: std::collections::HashMap<String, Profile>,
}

impl Config {
    pub fn read(root_dir: &Path) -> Result<Config> {
        let paths = [
            root_dir.join(".config").join("lunest.toml"),
            root_dir.join("lunest.toml"),
            root_dir.join(".lunest.toml"),
        ];
        let config = if let Some(s) = paths.iter().find_map(|p| std::fs::read_to_string(p).ok()) {
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

        anyhow::ensure!(
            !profile.lua.as_ref().unwrap().is_empty(),
            "lua command is empty"
        );

        Ok((name, profile))
    }

    pub fn group<'a>(&'a self, name: &'a str) -> Result<indexmap::IndexMap<&'a str, Profile>> {
        let mut profiles = indexmap::IndexMap::new();
        self.group_inner(name, &mut profiles, &mut std::collections::HashSet::new())?;
        Ok(profiles)
    }

    fn group_inner<'a>(
        &'a self,
        name: &'a str,
        profiles: &mut indexmap::IndexMap<&'a str, Profile>,
        visited_groups: &mut std::collections::HashSet<&'a str>,
    ) -> Result<()> {
        let members = self
            .group
            .get(name)
            .with_context(|| format!("group '{name}' is not defined"))?;
        if !visited_groups.insert(name) {
            return Ok(());
        }
        for member in members {
            if let Ok((s, p)) = self.profile(Some(member)) {
                profiles.insert(s, p);
            } else if self.group_inner(member, profiles, visited_groups).is_err() {
                anyhow::bail!("profile or group '{member}' is not defined");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn use_default_profile_if_empty() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(("default", Profile::default()), c.profile(None).unwrap());
    }

    #[test]
    fn detect_profile_from_one_profile() {
        let c: Config = toml::from_str(
            "[profile.a]
            init = 'a.lua'",
        )
        .unwrap();
        let (s, p) = c.profile(None).unwrap();
        assert_eq!(s, "a");
        assert_eq!(
            p,
            Profile {
                init: Some(PathBuf::from("a.lua")),
                ..Default::default()
            },
        );
    }

    #[test]
    fn detect_profile_from_multiple_profiles() {
        let c: Config = toml::from_str(
            "[profile.a]
            [profile.b]",
        )
        .unwrap();
        assert!(c.profile(None).is_err());
    }

    #[test]
    fn merge_default_profile() {
        let c: Config = toml::from_str(
            "[profile.default]
            init = 'a.lua'
            lua = ['lua']
            [profile.a]
            lua = ['lua5.1']",
        )
        .unwrap();
        let (s, p) = c.profile(Some("a")).unwrap();
        assert_eq!(s, "a");
        assert_eq!(
            p,
            Profile {
                init: Some(PathBuf::from("a.lua")),
                lua: Some(vec!["lua5.1".into()]),
                ..Default::default()
            }
        );
    }

    #[test]
    fn get_profiles_from_circular_referenced_group() {
        let c: Config = toml::from_str(
            "[group]
            a = ['b', 'd']
            b = ['a', 'c']
            [profile.c]
            [profile.d]",
        )
        .unwrap();
        assert_eq!(
            indexmap::indexmap! {
                "c" => Profile::default(),
                "d" => Profile::default(),
            },
            c.group("a").unwrap(),
        );
    }
}

#[derive(Clone, Debug, Deserialize, Merge, PartialEq)]
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

impl Profile {
    pub fn lua_command(
        &self,
        runtime_files: &mut crate::global::RuntimeFiles,
    ) -> std::io::Result<std::process::Command> {
        let lua = self.lua.as_ref().unwrap();
        let program = lua.first().unwrap(); // already validated in [`Config::profile`]
        let mut cmd = std::process::Command::new(runtime_files.get_lua_program(program)?);
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
