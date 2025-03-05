use anyhow::Context;
use merge::Merge;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct ConfigSpec {
    group: std::collections::HashMap<String, Vec<String>>,
    profile: std::collections::HashMap<String, crate::profile::Specifier>,
}

pub struct Config {
    root_dir: std::path::PathBuf,
    profiles: std::collections::HashMap<String, std::rc::Rc<crate::profile::Profile>>,
    groups: std::collections::HashMap<String, Vec<std::rc::Rc<crate::profile::Profile>>>,
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

    pub fn profile(&self, name: &str) -> anyhow::Result<&crate::profile::Profile> {
        let profile = self
            .profiles
            .get(name)
            .with_context(|| format!("profile '{name}' is not defined"))?;
        Ok(profile)
    }

    pub fn default_profile(&self) -> anyhow::Result<&crate::profile::Profile> {
        if let Some(profile) = self.profiles.get("default") {
            return Ok(profile);
        } else if self.profiles.len() == 1 {
            return Ok(self.profiles.values().next().unwrap());
        }
        anyhow::bail!("you must specify the profile or define a 'default' profile");
    }

    pub fn group(
        &self,
        name: &str,
    ) -> anyhow::Result<impl Iterator<Item = &crate::profile::Profile>> {
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
                spec.merge(crate::profile::Specifier::default());
                spec
            } else {
                crate::profile::Specifier::default()
            };
            for (name, mut spec) in config_spec.profile {
                spec.merge(default_spec.clone());
                let profile = crate::profile::Profile::from_spec(name.clone(), spec, &root_dir)?;
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
