pub use clap::Parser;

#[derive(Parser)]
#[command(name = "doer")]
#[command(version = "alpha")]
#[command(about = "KDL-based command management tools", long_about = None)]
pub struct Cli {
    /// Path to the KDL configuration file
    #[arg(short, long, default_value = "doer.kdl")]
    pub config: String,
    /// Name of the task to execute. If not provided, lists all available tasks.
    pub task: Option<String>,
    /// Parameters to pass to the task. Args are positional parameters, options are in the form of `key=value`.
    #[arg(allow_hyphen_values = true)]
    pub params: Vec<String>,
    /// Print the list of available tasks.
    #[arg(long)]
    pub tasks: bool,
}
