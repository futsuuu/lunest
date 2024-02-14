pub struct Cli {
    pub main_file: String,
    pub args: Args,
}

#[derive(clap::Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Run {
        #[arg(default_value = "default")]
        profile: String,
    },
    #[command(hide = true)]
    Test {
        #[arg(long)]
        profile: String,
        id: Vec<String>,
    },
}

impl Cli {
    pub fn new(args: Vec<String>) -> Self {
        use clap::Parser;
        Self {
            main_file: args[0].clone(),
            args: Args::parse_from(&args),
        }
    }
}
