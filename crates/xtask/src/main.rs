use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    xtask::Opt::parse().main()
}
