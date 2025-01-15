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
            include: Some(
                globset::GlobSet::builder()
                    .add(globset::Glob::new("{src,lua}/**/*.lua").unwrap())
                    .build()
                    .unwrap(),
            ),
            exclude: Some(globset::GlobSet::empty()),
            init: None,
        });
        default.clone()
    }
}

#[derive(Default)]
pub struct Config {
    profiles: std::collections::HashMap<String, std::rc::Rc<Profile>>,
    groups: std::collections::HashMap<String, Vec<std::rc::Rc<Profile>>>,
}

impl Config {
    pub fn read(root_dir: &std::path::Path) -> anyhow::Result<Self> {
        if let Some(path) = [root_dir.join("lunest.toml"), root_dir.join(".lunest.toml")]
            .into_iter()
            .find(|p| p.exists())
        {
            Self::from_spec(toml::from_str(&std::fs::read_to_string(path)?)?, root_dir)
        } else {
            Ok(Self::default())
        }
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

    fn from_spec(config_spec: ConfigSpec, root_dir: &std::path::Path) -> anyhow::Result<Self> {
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
                let profile = Profile::from_spec(name.clone(), spec, root_dir)?;
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
        Ok(Self { profiles, groups })
    }
}

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
        let lua = spec.lua.as_ref().unwrap();
        let target_files = target_files(
            &spec.include.unwrap_or_default(),
            &spec.exclude.unwrap_or_default(),
            root_dir,
        )?;
        Ok(Self {
            name,
            init_script: spec.init,
            target_files,
            lua_program: lua.first().context("'lua' field is empty")?.to_string(),
            lua_args: lua.get(1..).unwrap_or_default().to_vec(),
        })
    }
}

fn target_files(
    include: &globset::GlobSet,
    exclude: &globset::GlobSet,
    root_dir: &std::path::Path,
) -> std::io::Result<Vec<std::path::PathBuf>> {
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
            r.push(entry.into_path());
        }
    }
    Ok(r)
}
