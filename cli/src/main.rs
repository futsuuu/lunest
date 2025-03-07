mod app;
mod buffer;
mod command;
mod process;
mod profile;

use std::io::Write;

use clap::Parser;
use crossterm::style::Stylize;

#[tokio::main]
async fn main() -> anyhow::Result<std::process::ExitCode> {
    init_logger();
    log::info!("start");
    let code = match Args::parse() {
        Args::Run(c) => c.exec().await?,
        Args::List(c) => c.exec().await?,
        Args::Wrapper(c) => c.exec()?,
    };
    Ok(code)
}

fn init_logger() {
    env_logger::builder().format_timestamp_millis().init();
}

/// Lua testing framework
#[derive(Debug, clap::Parser)]
#[command(version, about)]
enum Args {
    /// Run tests
    #[command(visible_alias = "r")]
    Run(RunCommand),

    /// List tests
    #[command(visible_alias = "ls")]
    List(ListCommand),

    /// Print wrapper Lua code used for in-source testing
    Wrapper(WrapperCommand),
}

#[derive(clap::Args, Debug)]
struct RunCommand {
    #[clap(flatten)]
    app_options: app::Options,
}

impl RunCommand {
    async fn exec(self) -> anyhow::Result<std::process::ExitCode> {
        log::trace!("executing 'run' command");

        let app = app::App::new(self.app_options)?;

        let mut has_error = false;
        for (i, profile) in app.profiles().iter().enumerate() {
            if i != 0 {
                println!();
            }
            if !run(&app, profile).await? {
                has_error = true;
            }
        }
        Ok(if has_error {
            std::process::ExitCode::FAILURE
        } else {
            std::process::ExitCode::SUCCESS
        })
    }
}

async fn run(app: &app::App, profile: &profile::Profile) -> anyhow::Result<bool> {
    println!("run with profile '{}'", profile.name().bold());

    let mut process = process::Process::spawn(app, profile).await?;

    process
        .write(&process::Input::Initialize {
            root_dir: app.root_dir().to_path_buf(),
            target_files: profile
                .target_files()
                .iter()
                .map(|p| process::TargetFile::from_path(p.to_path_buf(), app.root_dir()))
                .collect(),
            term_width: crossterm::terminal::size().map_or(60, |size| size.0),
        })
        .await?;

    if let Some(script) = profile.init_script() {
        process
            .write(&process::Input::Execute(script.to_path_buf()))
            .await?;
    }

    let ids = get_test_list(&mut process)
        .await?
        .into_iter()
        .map(|info| info.id)
        .collect::<Vec<_>>();
    println!("found {} tests", ids.len());
    process
        .write(&process::Input::Run {
            test_id_filter: Some(ids),
            test_mode: process::TestMode::Run,
        })
        .await?;
    process.write(&process::Input::Finish).await?;

    let mut results = Vec::new();
    println!();

    loop {
        let Some(output) = process.read().await? else {
            if process.is_running()? {
                continue;
            } else {
                break;
            }
        };
        match output {
            process::Output::TestFinished(t) => {
                println!("{t}");
                results.push(t);
            }
            process::Output::TestStarted(t) => {
                print!("{t}{}", crossterm::cursor::MoveToColumn(0));
                _ = std::io::stdout().flush();
            }
            _ => (),
        }
    }

    let (success, error): (Vec<_>, Vec<_>) = results.iter().partition(|r| r.success());
    println!(
        "\nsuccess: {}, error: {}",
        success.len().to_string().green(),
        error.len().to_string().red(),
    );
    Ok(error.is_empty())
}

#[derive(clap::Args, Debug)]
struct ListCommand {
    #[clap(flatten)]
    app_context_options: app::Options,
}

impl ListCommand {
    async fn exec(self) -> anyhow::Result<std::process::ExitCode> {
        log::trace!("executing 'list' command");

        let app = app::App::new(self.app_context_options)?;

        for (i, profile) in app.profiles().iter().enumerate() {
            if i != 0 {
                println!();
            }
            list(&app, profile).await?;
        }
        Ok(std::process::ExitCode::SUCCESS)
    }
}

async fn list(app: &app::App, profile: &profile::Profile) -> anyhow::Result<()> {
    println!("run with profile '{}'", profile.name().bold());

    let mut process = process::Process::spawn(app, profile).await?;

    process
        .write(&process::Input::Initialize {
            root_dir: app.root_dir().to_path_buf(),
            target_files: profile
                .target_files()
                .iter()
                .map(|p| process::TargetFile::from_path(p.to_path_buf(), app.root_dir()))
                .collect(),
            term_width: crossterm::terminal::size().map_or(60, |size| size.0),
        })
        .await?;
    if let Some(script) = profile.init_script() {
        process
            .write(&process::Input::Execute(script.to_path_buf()))
            .await?;
    }

    println!();
    let test_list = get_test_list(&mut process).await?;
    for info in &test_list {
        println!("{info}");
    }

    Ok(())
}

async fn get_test_list(process: &mut process::Process) -> anyhow::Result<Vec<process::TestInfo>> {
    process
        .write(&process::Input::Run {
            test_id_filter: None,
            test_mode: process::TestMode::SendInfo,
        })
        .await?;

    let mut list = Vec::new();
    loop {
        let Some(output) = process.read().await? else {
            anyhow::ensure!(process.is_running()?);
            continue;
        };
        match output {
            process::Output::TestInfo(info) => {
                list.push(info);
            }
            process::Output::AllInputsRead => {
                break;
            }
            _ => (),
        }
    }

    Ok(list)
}

#[derive(clap::Args, Debug)]
struct WrapperCommand {
    /// Write code into the specified file
    #[arg(long, short, value_name = "FILE")]
    out: Option<std::path::PathBuf>,
}

impl WrapperCommand {
    fn exec(&self) -> anyhow::Result<std::process::ExitCode> {
        let source = concat!(
            "-- Code generated by `lunest wrapper`. DO NOT EDIT.\n",
            "---@diagnostic disable\n",
            include_str!("../../module/lunest/wrapper.lua")
        );
        if let Some(path) = &self.out {
            anyhow::ensure!(!path.exists(), "file already exists");
            std::fs::write(path, source)?;
        } else {
            print!("{}", source);
        }
        Ok(std::process::ExitCode::SUCCESS)
    }
}
