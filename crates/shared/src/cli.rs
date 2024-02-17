pub use clap::Parser;

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
    #[arg(long, short, global = true, default_value = "default")]
    pub profile: String,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Run,
    #[command(hide = true)]
    Test {
        id: Vec<String>,
    },
}
