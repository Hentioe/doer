pub mod prelude;
pub mod result;

use crate::prelude::*;
use colored::Colorize;
use doer_spec::Runnable;
use std::process::Stdio;
use tokio::process::Command;

pub async fn run_foreground(runnable: &Runnable) -> Result<()> {
    for command in &runnable.commands {
        let mut cmd = build_command(runnable, command)?;
        print_running(runnable, command, false);
        let status = cmd
            .stdin(runnable.stdin)
            .stdout(runnable.stdout)
            .stderr(runnable.stderr)
            .status()
            .await
            .context(format!("failed to execute task '{}'", runnable.name))?;

        ensure!(
            status.success(),
            "task '{}' exited with status {}",
            runnable.name,
            status.code().map_or_else(|| "unknown".to_string(), |c| c.to_string())
        );
    }

    Ok(())
}

pub async fn run_foreground_all(runnables: &[Runnable]) -> Result<()> {
    for runnable in runnables {
        run_foreground(runnable).await?;
    }
    Ok(())
}

pub async fn run_background(runnable: &Runnable) -> Result<tokio::process::Child> {
    let commands = &runnable.commands;
    let len = commands.len();

    ensure!(!commands.is_empty(), "task '{}' has no commands", runnable.name);

    for command in &commands[..len - 1] {
        let mut cmd = build_command(runnable, command)?;
        print_running(runnable, command, false);
        let status = cmd
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .with_context(|| format!("failed to execute task '{}'", runnable.name))?;

        ensure!(
            status.success(),
            "task '{}' exited with status {}",
            runnable.name,
            status.code().map_or_else(|| "unknown".to_string(), |c| c.to_string())
        );
    }

    let last = &commands[len - 1];
    let mut cmd = build_command(runnable, last)?;
    print_running(runnable, last, true);
    let child = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context(format!("failed to spawn task '{}'", runnable.name))?;

    Ok(child)
}

pub async fn run_background_all(runnables: &[Runnable]) -> Result<Vec<tokio::process::Child>> {
    let mut children = Vec::new();
    for runnable in runnables {
        children.push(run_background(runnable).await?);
    }
    Ok(children)
}

fn build_command(runnable: &Runnable, command: &str) -> Result<Command> {
    let mut cmd = if let Some(ref user) = runnable.user {
        let mut cmd = Command::new("sudo");
        cmd.arg("-u").arg(user);
        // 当指定用户时，环境变量通过参数传递（因为 sudo -u 会重置环境变量）
        for env_var in &runnable.env_vars {
            cmd.arg(format!("{}={}", env_var.name, env_var.value));
        }
        cmd.arg("sh").arg("-cu").arg(command);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-cu").arg(command);
        for env_var in &runnable.env_vars {
            cmd.env(&env_var.name, &env_var.value);
        }
        cmd
    };

    if let Some(ref cwd) = runnable.cwd {
        cmd.current_dir(cwd);
    }

    Ok(cmd)
}

fn print_running(runnable: &Runnable, command: &str, is_background: bool) {
    let prompt = if runnable.user.is_some() { '#' } else { '$' };
    let bg = if is_background { " &" } else { "" };
    let name_part = format!("[{}]", runnable.name).truecolor(120, 160, 200);
    let cmd_part = format!("{prompt}{bg} {command}").truecolor(160, 160, 160);
    let full = format!("{name_part} {cmd_part}").bold();
    eprintln!("{full}");
}

pub async fn run_all(runnables: &[Runnable]) -> Result<()> {
    let mut children = Vec::new();

    for runnable in runnables {
        if runnable.commands.is_empty() {
            continue;
        }
        if runnable.background {
            let child = run_background(runnable).await?;
            children.push(child);
        } else {
            run_foreground(runnable).await?;
        }
    }

    for mut child in children {
        let _ = child.wait().await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use doer_spec::{EnvVar, Runnable};
    use std::collections::HashSet;

    fn make_runnable(
        name: &str,
        command: &str,
        cwd: Option<&str>,
        env_vars: HashSet<EnvVar>,
        user: Option<&str>,
    ) -> Runnable {
        Runnable::builder()
            .name(name.to_string())
            .commands(vec![command.to_string()])
            .cwd(cwd.map(|s| s.to_string()))
            .env_vars(env_vars)
            .user(user.map(|s| s.to_string()))
            .background(false)
            .build()
    }

    fn args_list(runnable: &Runnable) -> (String, Vec<String>) {
        if let Some(ref user) = runnable.user {
            let mut args = vec!["-u".to_string(), user.clone()];
            for env_var in &runnable.env_vars {
                args.push(format!("{}={}", env_var.name, env_var.value));
            }
            args.push("sh".to_string());
            args.push("-cu".to_string());
            args.push(runnable.commands[0].clone());
            ("sudo".to_string(), args)
        } else {
            ("sh".to_string(), vec!["-cu".to_string(), runnable.commands[0].clone()])
        }
    }

    fn env(name: &str, value: &str) -> EnvVar {
        EnvVar {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn test_args_simple() {
        let r = make_runnable("test", "echo hello", None, HashSet::new(), None);
        let (prog, args) = args_list(&r);
        assert_eq!(prog, "sh");
        assert_eq!(args, vec!["-cu", "echo hello"]);
    }

    #[test]
    fn test_args_with_user() {
        let r = make_runnable("test", "echo hello", None, HashSet::new(), Some("root"));
        let (prog, args) = args_list(&r);
        assert_eq!(prog, "sudo");
        assert_eq!(args, vec!["-u", "root", "sh", "-cu", "echo hello"]);
    }

    #[test]
    fn test_args_with_user_and_env() {
        let mut env_vars = HashSet::new();
        env_vars.insert(env("FOO", "bar"));
        env_vars.insert(env("BAZ", "qux"));
        let r = make_runnable("test", "echo hello", None, env_vars, Some("root"));
        let (prog, args) = args_list(&r);
        assert_eq!(prog, "sudo");
        // env vars are inserted between -u <user> and sh -cu
        assert_eq!(args[0], "-u");
        assert_eq!(args[1], "root");
        // env vars in any order (HashSet is unordered)
        let env_args: Vec<&str> = args[2..4].iter().map(|s| s.as_str()).collect();
        assert!(env_args.contains(&"FOO=bar"));
        assert!(env_args.contains(&"BAZ=qux"));
        assert_eq!(&args[4..], &["sh", "-cu", "echo hello"]);
    }

    #[test]
    fn test_args_simple_with_env() {
        let mut env_vars = HashSet::new();
        env_vars.insert(env("FOO", "bar"));
        let r = make_runnable("test", "echo $FOO", None, env_vars, None);
        let (prog, args) = args_list(&r);
        assert_eq!(prog, "sh");
        assert_eq!(args, vec!["-cu", "echo $FOO"]);
    }
}
