use colored::Colorize;
use doer::cli::*;
use doer::prelude::*;
use doer_parser::config::{Config, OptValue, Task};
use doer_spec::error;
use std::collections::HashMap;
use std::path::PathBuf;

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(&cli.config)?;

    match cli.task {
        None => {
            println!("Usage: {PACKAGE_NAME} [TASK] [ARGS] [OPTS]...\n");
            println!("Available tasks:");
            print_task_list(&config.tasks);
        }
        Some(task_name) => {
            doer::git_hooks::ensure_hooks(&config)?;

            let mut args = Vec::new();
            let mut opt_overrides = HashMap::new();

            for param in &cli.params {
                if let Some(key) = param.strip_prefix("--") {
                    opt_overrides.insert(key.to_string(), OptValue::Bool(true));
                } else if let Some((key, value)) = param.split_once('=') {
                    opt_overrides.insert(key.to_string(), OptValue::String(value.to_string()));
                } else {
                    args.push(param.clone());
                }
            }

            let runnables = config.build_all(&task_name, &args, &opt_overrides)?;
            let has_commands = runnables.iter().any(|r| !r.commands.is_empty());
            if !has_commands {
                error!("task '{task_name}' has no command to run");
                return Ok(());
            }
            doer_runner::run_all(&runnables).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        report_error(&err);
        std::process::exit(1);
    }
}

fn print_task_list(tasks: &[Task]) {
    for task in tasks {
        let args_display =
            task.args.iter().map(|arg| format!("<{}>", arg)).collect::<Vec<_>>().join(" ").truecolor(240, 115, 60);
        let flag_opt_names: std::collections::HashSet<String> = task
            .opts
            .iter()
            .filter(|opt| {
                if let OptValue::String(s) = &opt.value {
                    s.strip_prefix('{').and_then(|s| s.strip_suffix('}')).is_some_and(|ref_name| {
                        task.opts.iter().any(|o| matches!(&o.value, OptValue::Bool(_)) && o.name == ref_name)
                    })
                } else {
                    false
                }
            })
            .map(|opt| opt.name.clone())
            .collect();
        let opts_display = task
            .opts
            .iter()
            .filter(|opt| !flag_opt_names.contains(&opt.name))
            .map(|opt| match &opt.value {
                OptValue::Bool(_) => format!("--{}", opt.name),
                OptValue::String(v) => format!("{}={}", opt.name, v),
            })
            .collect::<Vec<_>>()
            .join(" ")
            .truecolor(120, 160, 200);
        println!("  {} {args_display} {opts_display}", task.name);
    }
}

fn load_config(path: &str) -> Result<Config> {
    if !PathBuf::from(path).exists() {
        bail!("no '{PACKAGE_NAME}.kdl' found");
    }

    Config::load_from_kdl_file(path)
}

fn report_error(err: &anyhow::Error) {
    error!("{err}");
    for cause in err.chain().skip(1) {
        eprintln!("\t{}", cause);
    }
}
