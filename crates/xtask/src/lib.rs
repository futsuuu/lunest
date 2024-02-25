mod build;
mod test;

use std::process::Command;

use anyhow::Result;

use build::Build;
use test::Test;
use Lua::*;

#[derive(clap::Parser)]
pub enum Opt {
    #[command(alias = "b")]
    Build(Build),
    #[command(alias = "t")]
    Test(Test),
    Install(Build),
}

impl Opt {
    pub fn main(&self) -> Result<()> {
        match self {
            Opt::Build(build) => {
                build.build()?;
            }
            Opt::Test(test) => {
                test.test()?;
            }
            Opt::Install(build) => {
                build.install()?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, clap::Parser)]
pub struct Common {
    #[arg(
        long,
        short,
        value_enum,
        num_args = 1..,
        default_values_t = [Lua51, Lua52, Lua53, Lua54],
    )]
    lua: Vec<Lua>,
}

#[derive(Clone, clap::ValueEnum)]
enum Lua {
    Lua51,
    Lua52,
    Lua53,
    Lua54,
}

impl From<&Lua> for &'static str {
    fn from(value: &Lua) -> &'static str {
        match value {
            Lua51 => "lua51",
            Lua52 => "lua52",
            Lua53 => "lua53",
            Lua54 => "lua54",
        }
    }
}

fn sep(cmd: &Command) {
    println!(
        "\n──────────── {}",
        lunest_shared::utils::command_to_string(cmd)
    );
}
