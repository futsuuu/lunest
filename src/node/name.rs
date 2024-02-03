use std::{ffi::OsStr, path::PathBuf};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Name {
    Path(PathBuf),
    String(String),
}

impl Name {
    pub fn as_path(&self) -> Option<&PathBuf> {
        match self {
            Name::Path(p) => Some(p),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&String> {
        match self {
            Name::String(s) => Some(s),
            _ => None,
        }
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(p) => f.write_str(p.to_string_lossy().to_string().as_str()),
            Self::String(s) => f.write_str(s.as_str()),
        }
    }
}

impl AsRef<OsStr> for Name {
    fn as_ref(&self) -> &OsStr {
        match self {
            Self::Path(p) => p.as_os_str(),
            Self::String(s) => OsStr::new(s),
        }
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<Name> for String {
    fn from(value: Name) -> Self {
        match value {
            Name::Path(p) => p.to_string_lossy().to_string(),
            Name::String(s) => s,
        }
    }
}

impl From<PathBuf> for Name {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}
