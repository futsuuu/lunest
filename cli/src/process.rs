mod line_reader;

use std::fmt;

use crossterm::{style::Stylize, terminal};
use serde::{Deserialize, Serialize};

pub struct Process<R: std::io::Read, W: std::io::Write> {
    inner: std::process::Child,
    input: W,
    output: line_reader::LineReader<R>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{}", get_exit_error_message(.0))]
    Exit(Option<i32>),
}

impl Process<std::fs::File, std::fs::File> {
    pub fn spawn(
        cx: &crate::global::Context,
        profile: &crate::config::Profile,
    ) -> Result<Self, std::io::Error> {
        let temp_dir = cx.create_process_dir()?;
        let input_path = temp_dir.join("in.jsonl");
        let output_path = temp_dir.join("out.jsonl");
        Ok(Self {
            inner: {
                profile
                    .lua_command(cx)?
                    .arg(cx.get_main_script())
                    .env("LUNEST_IN", &input_path)
                    .env("LUNEST_OUT", &output_path)
                    .spawn()?
            },
            input: std::fs::File::options()
                .create_new(true)
                .append(true)
                .open(input_path)?,
            output: line_reader::LineReader::new({
                std::fs::write(&output_path, "")?;
                std::fs::File::open(output_path)?
            }),
        })
    }
}

impl<R: std::io::Read, W: std::io::Write> Process<R, W> {
    pub fn read(&mut self) -> Result<Option<Output>, std::io::Error> {
        match self.output.read_line() {
            Ok(s) => Ok(Some(
                serde_json::from_str(&s).expect("failed to deserialize a response"),
            )),
            Err(line_reader::Error::Io(e)) => Err(e),
            Err(line_reader::Error::NoNewLine) => Ok(None),
            Err(line_reader::Error::Empty) => Ok(None),
        }
    }

    pub fn write(&mut self, req: &Input) -> Result<(), std::io::Error> {
        let mut json = serde_json::to_vec(req).expect("failed to serialize a request");
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

impl<R: std::io::Read, W: std::io::Write> Drop for Process<R, W> {
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
pub enum Input {
    Initialize {
        init_file: Option<std::path::PathBuf>,
        root_dir: std::path::PathBuf,
        target_files: Vec<TargetFile>,
        term_width: u16,
    },
}

#[derive(Serialize)]
pub struct TargetFile {
    name: String,
    path: std::path::PathBuf,
}

impl TargetFile {
    pub fn new(path: std::path::PathBuf, root_dir: &std::path::Path) -> Self {
        let relative_path = path.strip_prefix(root_dir).unwrap_or(&path);
        let name = relative_path.display().to_string().replace('\\', "/");
        Self { path, name }
    }
}

#[derive(Deserialize)]
pub enum Output {
    TestStarted(TestStarted),
    TestFinished(TestFinished),
}

fn fmt_title(title: &[String]) -> String {
    title.join(&" :: ".grey().to_string())
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
