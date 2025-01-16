use std::{fmt, io::Write};

use crossterm::{style::Stylize, terminal};
use serde::{Deserialize, Serialize};

pub struct Process {
    inner: std::process::Child,
    input: std::fs::File,
    output: crate::io::LineBufReader<std::fs::File>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{}", get_exit_error_message(.0))]
    Exit(Option<i32>),
}

impl Process {
    pub fn spawn(
        cx: &crate::global::Context,
        profile: &crate::config::Profile,
    ) -> Result<Self, std::io::Error> {
        let temp_dir = cx.create_process_dir()?;
        let input_path = temp_dir.join("in.jsonl");
        let output_path = temp_dir.join("out.jsonl");
        Ok(Self {
            inner: profile
                .lua_command(cx)?
                .arg(cx.get_main_script())
                .env("LUNEST_IN", &input_path)
                .env("LUNEST_OUT", &output_path)
                .current_dir(cx.root_dir())
                .spawn()?,
            input: std::fs::File::options()
                .create_new(true)
                .append(true)
                .open(input_path)?,
            output: crate::io::LineBufReader::new({
                std::fs::write(&output_path, "")?;
                std::fs::File::open(output_path)?
            }),
        })
    }

    pub fn read(&mut self) -> Result<Option<Output>, std::io::Error> {
        match self.output.read_line()? {
            crate::io::Line::Ok(s) => Ok(Some(
                serde_json::from_str(&s).expect("failed to deserialize an output"),
            )),
            crate::io::Line::NoLF => Ok(None),
            crate::io::Line::Empty => Ok(None),
        }
    }

    pub fn write(&mut self, input: &Input) -> Result<(), std::io::Error> {
        let mut json = serde_json::to_vec(input).expect("failed to serialize an input");
        json.extend(b"\n");
        self.input.write_all(&json)
    }

    pub fn is_running(&mut self) -> Result<bool, Error> {
        match self.inner.try_wait()? {
            Some(status) => match status.code() {
                Some(0) => Ok(false),
                code => Err(Error::Exit(code)),
            },
            None => Ok(true),
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        _ = self.inner.kill();
    }
}

fn get_exit_error_message(code: &Option<i32>) -> String {
    match code {
        Some(n) => format!("spawned process exited with status code {n}"),
        None => "spawned process terminated by signal".into(),
    }
}

#[derive(Serialize)]
#[serde(tag = "t", content = "c")]
pub enum Input {
    Initialize {
        mode: Mode,
        root_dir: std::path::PathBuf,
        term_width: u16,
    },
    TestFile {
        path: std::path::PathBuf,
        name: String,
    },
    Execute(std::path::PathBuf),
    Finish,
}

#[derive(Serialize)]
pub enum Mode {
    Run,
    List,
}

#[derive(Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum Output {
    TestFound(TestFound),
    TestStarted(TestStarted),
    TestFinished(TestFinished),
}

fn fmt_title(title: &[String]) -> String {
    title.join(&" :: ".grey().to_string())
}

#[derive(Deserialize)]
pub struct TestFound {
    title: Vec<String>,
}

impl fmt::Display for TestFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", fmt_title(&self.title))
    }
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
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

#[derive(Deserialize)]
pub struct TestError {
    message: String,
    traceback: String,
    info: Option<TestErrorInfo>,
}

#[derive(Deserialize)]
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
