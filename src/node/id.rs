use std::path::PathBuf;

use anyhow::{Context as _, Result};

use super::Name;

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct ID {
    path: Option<PathBuf>,
    names: Vec<String>,
}

impl ID {
    const SEPARATOR: &'static str = " ┃ ";

    pub fn root() -> Self {
        Self {
            path: None,
            names: Vec::new(),
        }
    }

    pub fn get(&self, index: usize) -> Option<Name> {
        if index == 0 {
            self.path.as_ref().map(|p| Name::from(p.to_path_buf()))
        } else {
            self.names.get(index - 1).map(|s| Name::from(s.to_string()))
        }
    }

    pub fn name(&self) -> Option<Name> {
        self.get(self.names.len())
    }

    pub fn is_root(&self) -> bool {
        self.get(0).is_none()
    }

    pub fn push(&mut self, item: &Name) -> Result<()> {
        let context = "ID must start with a path and be followed by a string";
        if self.path.is_none() {
            self.path = Some(
                item.as_path()
                    .with_context(|| format!("`{item}` is not a path: {context}"))?
                    .to_path_buf(),
            );
        } else {
            self.names.push(
                item.as_string()
                    .with_context(|| format!("`{item}` is not a string: {context}"))?
                    .to_string(),
            );
        }
        Ok(())
    }

    pub fn pop(&mut self) {
        if self.names.pop().is_none() {
            self.path = None;
        }
    }
}

#[cfg(test)]
mod methods {
    use super::*;

    #[test]
    fn get() {
        let id = ID {
            path: Some(PathBuf::from("path")),
            names: vec!["first".into(), "second".into()],
        };
        assert_eq!(Some(Name::from(PathBuf::from("path"))), id.get(0));
        assert_eq!(Some(Name::from(String::from("first"))), id.get(1));
        assert_eq!(Some(Name::from(String::from("second"))), id.get(2));
        assert_eq!(None, id.get(3));
    }
}

impl IntoIterator for ID {
    type Item = Name;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let names: Vec<Name> = self.into();
        names.into_iter()
    }
}

impl std::fmt::Display for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let strings: Vec<String> = self.clone().into();
        f.write_str(strings.join(Self::SEPARATOR).as_str())
    }
}

impl From<Vec<String>> for ID {
    fn from(value: Vec<String>) -> Self {
        Self {
            path: value.first().map(PathBuf::from),
            names: value.get(1..).unwrap_or_default().to_vec(),
        }
    }
}

#[cfg(test)]
mod from_vec {
    use super::*;

    #[test]
    fn empty() {
        let id = ID::from(Vec::new());
        assert_eq!(
            ID {
                path: None,
                names: vec![]
            },
            id
        );
    }

    #[test]
    fn only_path() {
        let id = ID::from(vec!["path".to_string()]);
        assert_eq!(
            ID {
                path: Some(PathBuf::from("path")),
                names: vec![]
            },
            id
        )
    }

    #[test]
    fn with_names() {
        let id = ID::from(vec![
            "path".to_string(),
            "first".to_string(),
            "second".to_string(),
        ]);
        assert_eq!(
            ID {
                path: Some(PathBuf::from("path")),
                names: vec![String::from("first"), String::from("second")]
            },
            id
        );
    }
}

impl<T: From<Name>> From<ID> for Vec<T> {
    fn from(value: ID) -> Self {
        let Some(path) = value.path else {
            return Vec::new();
        };
        let mut vec = Vec::new();
        vec.push(Name::from(path).into());
        for name in value.names {
            vec.push(Name::from(name).into());
        }
        vec
    }
}

#[cfg(test)]
mod into_vec {
    use super::*;

    #[test]
    fn empty() {
        let id = ID {
            path: None,
            names: vec![],
        };
        let vec: Vec<String> = id.into();
        assert!(vec.is_empty());
    }

    #[test]
    fn only_path() {
        let id = ID {
            path: Some(PathBuf::from("path")),
            names: vec![],
        };
        let vec: Vec<String> = id.into();
        assert_eq!(vec![String::from("path")], vec);
    }

    #[test]
    fn with_names() {
        let id = ID {
            path: Some(PathBuf::from("path")),
            names: vec!["first".into(), "second".into()],
        };

        let vec: Vec<String> = id.into();
        assert_eq!(
            vec![
                String::from("path"),
                String::from("first"),
                String::from("second"),
            ],
            vec
        );
    }
}
