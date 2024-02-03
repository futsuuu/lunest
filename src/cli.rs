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
        #[arg(long, short, num_args = 1.., default_values_t = [
            String::from(r"{test,spec}/**/*.lua"),
            String::from(r"*[-_\.]{test,spec}.lua"),
        ])]
        pattern: Vec<String>,
        #[arg(long, default_value = "lua", num_args = 1.., allow_hyphen_values = true)]
        lua_cmd: Vec<String>,
    },
    #[command(hide = true)]
    Test { id: Vec<String> },
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
