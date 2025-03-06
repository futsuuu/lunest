use std::fmt;

use crossterm::{style::Stylize, terminal};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

pub struct Process {
    inner: Option<std::process::Child>,
    input: tokio::fs::File,
    output: crate::buffer::AsyncLineReader<tokio::fs::File>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{}", get_exit_error_message(.0))]
    Exit(Option<i32>),
}

impl Process {
    pub async fn spawn(
        cx: &crate::global::Context,
        profile: &crate::profile::Profile,
    ) -> Result<Self, std::io::Error> {
        log::trace!("spawning new process");

        let temp_dir = cx.create_process_dir()?;
        let input_path = temp_dir.join("in.jsonl");
        let output_path = temp_dir.join("out.jsonl");

        let mut cmd = profile.lua_command().clone();
        cmd.program(cx.get_lua_program(cmd.get_program())?)
            .arg(cx.get_main_script())
            .env("LUNEST_IN", &input_path)
            .env("LUNEST_OUT", &output_path)
            .current_dir(cx.root_dir());
        log::debug!("lua command: {}", cmd.display().env(true));

        let child = loop {
            match cmd
                .build()
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(child) => {
                    break child;
                }
                Err(e) if e.kind() == std::io::ErrorKind::ResourceBusy => {
                    log::warn!("failed to spawn the command: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    log::info!("retrying to spawn...");
                }
                Err(e) => {
                    log::error!("failed to spawn the command: {e}");
                    return Err(e);
                }
            }
        };
        log::info!("process spawned as {}", child.id());

        Ok(Self {
            inner: Some(child),
            input: tokio::fs::File::options()
                .create_new(true)
                .append(true)
                .open(input_path)
                .await?,
            output: {
                tokio::fs::File::create(&output_path).await?;
                tokio::fs::File::open(output_path).await?.into()
            },
        })
    }

    pub async fn read(&mut self) -> Result<Option<Output>, std::io::Error> {
        let output = match self.output.read_line().await? {
            crate::buffer::Line::Ok(s) => {
                let out = serde_json::from_str(&s).expect("failed to deserialize an output");
                match &out {
                    Output::Log(s) => log::info!("[log] {s}"),
                    _ => log::debug!("output read: {out:?}"),
                }
                Some(out)
            }
            crate::buffer::Line::NoLF | crate::buffer::Line::Empty => None,
        };
        Ok(output)
    }

    pub async fn write(&mut self, input: &Input) -> Result<(), std::io::Error> {
        log::debug!("writing input: {input:?}");
        let mut json = serde_json::to_vec(input).expect("failed to serialize an input");
        json.extend(b"\n");
        self.input.write_all(&json).await?;
        Ok(())
    }

    pub fn is_running(&mut self) -> Result<bool, Error> {
        let Some(inner) = &mut self.inner else {
            return Ok(true);
        };
        if inner.try_wait()?.is_none() {
            return Ok(true);
        }
        let inner = self.inner.take().unwrap();
        log::info!("process {} already exited", inner.id());

        let out = inner.wait_with_output()?;
        log::debug!("stdout: {}", String::from_utf8_lossy(&out.stdout));
        log::debug!("stderr: {}", String::from_utf8_lossy(&out.stderr));

        match out.status.code() {
            Some(0) => Ok(false),
            code => Err(Error::Exit(code)),
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Some(inner) = &mut self.inner {
            _ = inner.kill();
        }
    }
}

fn get_exit_error_message(code: &Option<i32>) -> String {
    match code {
        Some(n) => format!("spawned process exited with status code {n}"),
        None => "spawned process terminated by signal".into(),
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "t", content = "c")]
pub enum Input {
    Initialize {
        target_files: Vec<TargetFile>,
        root_dir: std::path::PathBuf,
        term_width: u16,
    },
    Run {
        test_id_filter: Option<Vec<String>>,
        test_mode: TestMode,
    },
    Execute(std::path::PathBuf),
    Finish,
}

#[derive(Debug, Serialize)]
pub struct TargetFile {
    path: std::path::PathBuf,
    name: String,
}

#[derive(Debug, Serialize)]
pub enum TestMode {
    Run,
    SendInfo,
}

impl TargetFile {
    pub fn from_path(path: std::path::PathBuf, root_dir: &std::path::Path) -> Self {
        let rel = path.strip_prefix(root_dir).unwrap_or(&path);
        let name = rel.display().to_string().replace('\\', "/");
        Self { path, name }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum Output {
    TestInfo(TestInfo),
    TestStarted(TestStarted),
    TestFinished(TestFinished),
    AllInputsRead,
    Log(String),
}

fn fmt_title(title: &[String]) -> String {
    title.join(&" :: ".grey().to_string())
}

#[derive(Debug, Deserialize)]
pub struct TestInfo {
    pub id: String,
    pub title: Vec<String>,
}

impl fmt::Display for TestInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", fmt_title(&self.title))
    }
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
