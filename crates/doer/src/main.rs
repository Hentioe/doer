use colored::Colorize;
use doer::cli::*;
use doer::prelude::*;
use doer_parser::Config;
use std::collections::HashMap;
use std::path::PathBuf;

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(&cli.config)?;

    match cli.task {
        None => {
            println!("Usage: {PACKAGE_NAME} [TASK] [ARGS] [OPTS]...\n");
            println!("Available tasks:");
            for task in &config.tasks {
                let args_display = task
                    .args
                    .iter()
                    .map(|arg| format!("<{}>", arg))
                    .collect::<Vec<_>>()
                    .join(" ")
                    // 显示为浅红
                    .truecolor(240, 115, 60);
                let opts_display = task
                    .opts
                    .iter()
                    .map(|opt| format!("{}={}", opt.name, opt.value))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .truecolor(120, 160, 200);
                println!("  {} {args_display} {opts_display}", task.name);
            }
        }
        Some(task_name) => {
            let mut args = Vec::new();
            let mut opt_overrides = HashMap::new();

            for param in &cli.params {
                if let Some((key, value)) = param.split_once('=') {
                    opt_overrides.insert(key.to_string(), value.to_string());
                } else {
                    args.push(param.clone());
                }
            }

            let runnables = config.build_all(&task_name, &args, &opt_overrides)?;
            let has_commands = runnables.iter().any(|r| !r.commands.is_empty());
            if !has_commands {
                eprintln!("task '{task_name}' has no command to run");
                return Ok(());
            }
            doer_runner::run_all(&runnables).await?;
        }
    }

    Ok(())
}

fn load_config(path: &str) -> Result<Config> {
    if !PathBuf::from(path).exists() {
        bail!("no {PACKAGE_NAME}.kdl found");
    }

    Config::load_from_kdl_file(path)
}
