use std::{
    fmt, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;
use yansi::Paint;

#[derive(Debug)]
pub struct Bridge {
    path: PathBuf,
    reader: BufReader<fs::File>,
}

impl Bridge {
    pub fn new(temp_dir: &Path) -> Result<Self> {
        let path = temp_dir.join("messages");
        fs::write(&path, "")?;
        let reader = BufReader::new(fs::File::open(&path)?);
        Ok(Self { path, reader })
    }

    pub fn read(&mut self) -> Result<Option<Message>> {
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim();
        let msg: Message = serde_json::from_str(line)
            .with_context(|| format!("failed to deserialize json: {line}"))?;
        Ok(Some(msg))
    }

    pub fn overwrite_main_lua(
        &self,
        contents: &str,
        root_dir: impl AsRef<Path>,
        target_files: impl IntoIterator<Item = impl AsRef<Path>>,
        init_file: Option<impl AsRef<Path>>,
    ) -> String {
        let root_dir = root_dir.as_ref();
        let files: String = target_files.into_iter().fold(String::new(), |acc, p| {
            let p = p.as_ref();
            let name = p.strip_prefix(root_dir).unwrap_or(p);
            format!(
                "{acc}{{ name = \"{}\", path = \"{}\" }}, ",
                name.display().to_string().replace('\\', "/"),
                p.display().to_string().replace('\\', r"\\"),
            )
        });
        let contents = contents
            .replace(
                "local TARGET_FILES\n",
                &format!("local TARGET_FILES = {{ {files} }}\n"),
            )
            .replace(
                "local MSG_FILE\n",
                &format!(
                    "local MSG_FILE = \"{}\"\n",
                    self.path.display().to_string().replace('\\', r"\\")
                ),
            );
        if let Some(path) = init_file {
            contents.replace(
                "local INIT_FILE\n",
                &format!(
                    "local INIT_FILE = \"{}\"\n",
                    path.as_ref().display().to_string().replace('\\', r"\\"),
                ),
            )
        } else {
            contents
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum Message {
    TestResult(TestResult),
}

#[derive(Debug, Deserialize)]
pub struct TestResult {
    title: Vec<String>,
    error: Option<TestError>,
}

#[derive(Debug, Deserialize)]
pub enum TestError {
    Msg(String),
}

impl TestResult {
    pub fn success(&self) -> bool {
        self.error.is_none()
    }
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{} ",
            self.title.join(&" :: ".dim().to_string()),
            ":".dim()
        )?;
        if let Some(err) = &self.error {
            write!(f, "{}\n{}", "ERR".red().bold(), err)
        } else {
            write!(f, "{}", "OK".green().bold())
        }
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            TestError::Msg(msg) => write!(f, "{msg}"),
        }
    }
}
