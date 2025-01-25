mod config;
mod global;
mod io;
mod process;

use std::io::Write;

use clap::Parser;
use crossterm::style::Stylize;

fn main() -> anyhow::Result<()> {
    init_logger();
    log::info!("start");
    match Args::parse() {
        Args::Run(c) => c.exec(),
        Args::List(c) => c.exec(),
        Args::Wrapper(c) => c.exec(),
    }
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
    profiles: Profiles,
}

impl RunCommand {
    fn exec(&self) -> anyhow::Result<()> {
        log::trace!("executing 'run' command");

        let cx = global::Context::new()?;

        let mut has_error = false;
        for (i, profile) in self.profiles.collect(cx.config())?.iter().enumerate() {
            if i != 0 {
                println!();
            }
            if !run(&cx, profile)? {
                has_error = true;
            }
        }
        if has_error {
            std::process::exit(1);
        }
        Ok(())
    }
}

fn run(cx: &global::Context, profile: &config::Profile) -> anyhow::Result<bool> {
    println!("run with profile '{}'", profile.name().bold());

    let mut process = process::Process::spawn(cx, profile)?;

    process.write(&process::Input::Initialize {
        root_dir: cx.root_dir().to_path_buf(),
        target_files: profile
            .target_files()
            .iter()
            .map(|p| process::TargetFile::from_path(p.to_path_buf(), cx.root_dir()))
            .collect(),
        term_width: crossterm::terminal::size().map_or(60, |size| size.0),
    })?;

    if let Some(script) = profile.init_script() {
        process.write(&process::Input::Execute(script.to_path_buf()))?;
    }

    let ids = get_test_list(&mut process)?
        .into_iter()
        .map(|info| info.id)
        .collect::<Vec<_>>();
    println!("found {} tests", ids.len());
    process.write(&process::Input::Run {
        test_id_filter: Some(ids),
        test_mode: process::TestMode::Run,
    })?;
    process.write(&process::Input::Finish)?;

    let mut results = Vec::new();
    println!();

    loop {
        let Some(output) = process.read()? else {
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
    profiles: Profiles,
}

impl ListCommand {
    fn exec(&self) -> anyhow::Result<()> {
        log::trace!("executing 'list' command");

        let cx = global::Context::new()?;

        for (i, profile) in self.profiles.collect(cx.config())?.iter().enumerate() {
            if i != 0 {
                println!();
            }
            list(&cx, profile)?;
        }
        Ok(())
    }
}

fn list(cx: &global::Context, profile: &config::Profile) -> anyhow::Result<()> {
    println!("run with profile '{}'", profile.name().bold());

    let mut process = process::Process::spawn(cx, profile)?;

    process.write(&process::Input::Initialize {
        root_dir: cx.root_dir().to_path_buf(),
        target_files: profile
            .target_files()
            .iter()
            .map(|p| process::TargetFile::from_path(p.to_path_buf(), cx.root_dir()))
            .collect(),
        term_width: crossterm::terminal::size().map_or(60, |size| size.0),
    })?;
    if let Some(script) = profile.init_script() {
        process.write(&process::Input::Execute(script.to_path_buf()))?;
    }

    println!();
    let test_list = get_test_list(&mut process)?;
    for info in &test_list {
        println!("{info}");
    }

    Ok(())
}

fn get_test_list(process: &mut process::Process) -> anyhow::Result<Vec<process::TestInfo>> {
    process.write(&process::Input::Run {
        test_id_filter: None,
        test_mode: process::TestMode::SendInfo,
    })?;

    let mut list = Vec::new();
    loop {
        let Some(output) = process.read()? else {
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
struct Profiles {
    /// Load Lua files with the specified profile
    #[arg(long, short, value_delimiter = ',')]
    profile: Vec<String>,
    /// Load Lua files with the profiles in the specified group
    #[arg(long, short, value_delimiter = ',')]
    group: Vec<String>,
}

impl Profiles {
    fn collect<'a>(
        &'a self,
        config: &'a config::Config,
    ) -> anyhow::Result<Vec<&'a config::Profile>> {
        let mut ps = Vec::new();
        for profile in &self.profile {
            ps.push(config.profile(profile)?);
        }
        for group in &self.group {
            ps.extend(config.group(group)?);
        }
        if ps.is_empty() {
            ps.push(config.default_profile()?);
        }
        Ok(ps)
    }
}

#[derive(clap::Args, Debug)]
struct WrapperCommand {
    /// Write code into the specified file
    #[arg(long, short, value_name = "FILE")]
    out: Option<std::path::PathBuf>,
}

impl WrapperCommand {
    fn exec(&self) -> anyhow::Result<()> {
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
        Ok(())
    }
}
