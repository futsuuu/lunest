use std::{
    fs,
    path::{Path, PathBuf}, process,
};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    lua: Vec<String>,
    files: Vec<String>,
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

    pub fn lua_command(&self) -> Result<process::Command> {
        let mut cmd = process::Command::new(self.lua.first().context("command is empty")?);
        cmd.args(self.lua.get(1..).unwrap_or_default());
        Ok(cmd)
    }

    pub fn target_files(&self, root_dir: &Path) -> Result<Vec<PathBuf>> {
        let set = {
            let mut builder = globset::GlobSet::builder();
            for pat in &self.files {
                builder.add(globset::Glob::new(pat)?);
            }
            builder.build()?
        };
        let mut r = Vec::new();
        for entry in walkdir::WalkDir::new(root_dir).sort_by_file_name() {
            let entry = entry?;
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_file() && set.is_match(entry.path().strip_prefix(root_dir).unwrap()) {
                r.push(entry.into_path());
            }
        }
        Ok(r)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lua: vec!["lua".into()],
            files: vec!["{lua,src}/**/*.lua".into()],
        }
    }
}
