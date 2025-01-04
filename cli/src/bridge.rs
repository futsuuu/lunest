use std::{
    fmt, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use crossterm::{style::Stylize, terminal};
use serde::Deserialize;

#[derive(Debug)]
pub struct Bridge {
    path: PathBuf,
    reader: BufReader<fs::File>,
    line: String,
}

impl Bridge {
    pub fn new(temp_dir: &Path) -> Result<Self> {
        let path = temp_dir.join("messages.jsonl");
        fs::write(&path, "")?;
        let reader = BufReader::new(fs::File::open(&path)?);
        Ok(Self {
            path,
            reader,
            line: String::new(),
        })
    }

    pub fn read(&mut self) -> Result<Option<Message>> {
        while !self.line.ends_with('\n') {
            if self.reader.read_line(&mut self.line)? == 0 {
                return Ok(None);
            }
        }
        let msg: Message = {
            let line = self.line.trim_end_matches('\n');
            serde_json::from_str(line)
                .with_context(|| format!("failed to deserialize json: {line}"))?
        };
        self.line.clear();
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
                "--[[@replace = lunest.ROOT_DIR]]",
                &format!(
                    "= \"{}\"",
                    root_dir.display().to_string().replace('\\', r"\\")
                ),
            )
            .replace(
                "--[[@replace = lunest.TARGET_FILES]]",
                &format!("= {{ {files} }}"),
            )
            .replace(
                "--[[@replace = lunest.MSG_FILE]]",
                &format!(
                    "= \"{}\"",
                    self.path.display().to_string().replace('\\', r"\\")
                ),
            )
            .replace(
                "--[[@replace = lunest.TERM_WIDTH]]",
                &format!(
                    "= {}",
                    crossterm::terminal::size().map_or(60, |size| size.0)
                ),
            );
        if let Some(path) = init_file {
            contents.replace(
                "--[[@replace = lunest.INIT_FILE]]",
                &format!(
                    "= \"{}\"",
                    path.as_ref().display().to_string().replace('\\', r"\\"),
                ),
            )
        } else {
            contents
        }
    }
}

fn fmt_title(title: &[String]) -> String {
    title.join(&" :: ".grey().to_string())
}

#[derive(Debug, Deserialize)]
pub enum Message {
    TestStarted(TestStarted),
    TestFinished(TestFinished),
}

#[derive(Debug, Deserialize)]
pub struct TestStarted {
    title: Vec<String>,
}

impl fmt::Display for TestStarted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{} {}",
            fmt_title(&self.title),
            ":".grey(),
            "RUNNING".cyan().bold()
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct TestFinished {
    title: Vec<String>,
    error: Option<TestError>,
}

impl TestFinished {
    pub fn success(&self) -> bool {
        self.error.is_none()
    }
}

impl fmt::Display for TestFinished {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", terminal::Clear(terminal::ClearType::UntilNewLine))?;
        write!(f, "{}{} ", fmt_title(&self.title), ":".grey())?;
        if let Some(err) = &self.error {
            write!(f, "{}\n{}", "ERR".red().bold(), err)
        } else {
            write!(f, "{}", "OK".green().bold())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TestError {
    message: String,
    traceback: String,
    info: Option<TestErrorInfo>,
}

#[derive(Debug, Deserialize)]
pub enum TestErrorInfo {
    Diff { left: String, right: String },
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}\n", self.message.as_str().bold())?;
        if let Some(info) = &self.info {
            writeln!(f, "{info}")?;
        }
        writeln!(f, "{}:", "  stack traceback".bold())?;
        writeln!(f, "{}", self.traceback)?;
        Ok(())
    }
}

impl fmt::Display for TestErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            TestErrorInfo::Diff { left, right } => {
                writeln!(
                    f,
                    "{} ({} {} {}):",
                    "  difference".bold(),
                    "-left".red(),
                    "/".grey(),
                    "+right".green()
                )?;
                let delete = "-".red();
                let insert = "+".green();
                let diff = similar::TextDiff::from_lines(left, right);
                for change in diff.iter_all_changes() {
                    let line = change.value();
                    use similar::ChangeTag::*;
                    match change.tag() {
                        Equal => write!(f, " {}", line.grey())?,
                        Delete => write!(f, "{delete}{}", line.red())?,
                        Insert => write!(f, "{insert}{}", line.green())?,
                    }
                    if !line.ends_with('\n') {
                        writeln!(f)?;
                    }
                }
            }
        }
        Ok(())
    }
}
