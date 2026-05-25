pub mod prelude;
pub mod result;

use crate::prelude::*;
use colored::Colorize;
use doer_spec::{Runnable, warn};
use std::borrow::Cow;
use std::process::Stdio;
use tokio::process::Command;

/// Automatically choose the nice application method based on the runnable's user:
/// - If a user is set (task runs via `sudo`), prepend `nice -n <val>` to the command
///   string so it executes inside the escalated shell context (Shell method).
/// - If no user is set (task runs as the current process), call `libc::setpriority`
///   directly on the child PID (Libc method), which is more direct.
fn use_shell_nice(runnable: &Runnable) -> bool {
    runnable.user.is_some()
}

fn set_nice(pid: u32, nice_val: i32) -> Result<()> {
    let ret = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid, nice_val) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        warn!("failed to set nice value {nice_val} on pid {pid}: {}", err.kind());
    }
    Ok(())
}

async fn spawn_and_wait(runnable: &Runnable, command: &str, is_background: bool) -> Result<std::process::ExitStatus> {
    let mut cmd = build_command(runnable, command)?;
    print_running(runnable, command, is_background);
    let (stdin, stdout, stderr) = if is_background {
        (Stdio::inherit(), Stdio::inherit(), Stdio::inherit())
    } else {
        (runnable.stdin.into(), runnable.stdout.into(), runnable.stderr.into())
    };
    let mut child = cmd
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .context(format!("failed to spawn task '{}'", runnable.name))?;

    if !use_shell_nice(runnable)
        && let Some(nice_val) = runnable.nice
    {
        set_nice(child.id().context("failed to get child pid")?, nice_val)?;
    }

    let status = child.wait().await.context(format!("failed to wait for task '{}'", runnable.name))?;
    Ok(status)
}

pub async fn run_foreground(runnable: &Runnable) -> Result<()> {
    for command in &runnable.commands {
        let status = spawn_and_wait(runnable, command, false).await?;
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
        let status = spawn_and_wait(runnable, command, false).await?;
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

    if !use_shell_nice(runnable)
        && let Some(nice_val) = runnable.nice
    {
        set_nice(child.id().context("failed to get child pid")?, nice_val)?;
    }

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
    let cmd_str = match runnable.nice {
        Some(nice_val) if use_shell_nice(runnable) => Cow::Owned(format!("nice -n {nice_val} {command}")),
        _ => Cow::Borrowed(command),
    };
    let mut cmd = if let Some(ref user) = runnable.user {
        let mut cmd = Command::new("sudo");
        cmd.arg("-u").arg(user);
        // 当指定用户时，环境变量通过参数传递（因为 sudo -u 会重置环境变量）
        for env_var in &runnable.env_vars {
            cmd.arg(format!("{}={}", env_var.name, env_var.value));
        }
        cmd.arg("sh").arg("-cu").arg(cmd_str.as_ref());
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-cu").arg(cmd_str.as_ref());
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
        let cmd = match runnable.nice {
            Some(nice_val) if runnable.user.is_some() => {
                format!("nice -n {nice_val} {}", runnable.commands[0])
            }
            _ => runnable.commands[0].clone(),
        };
        if let Some(ref user) = runnable.user {
            let mut args = vec!["-u".to_string(), user.clone()];
            for env_var in &runnable.env_vars {
                args.push(format!("{}={}", env_var.name, env_var.value));
            }
            args.push("sh".to_string());
            args.push("-cu".to_string());
            args.push(cmd);
            ("sudo".to_string(), args)
        } else {
            ("sh".to_string(), vec!["-cu".to_string(), cmd])
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
        assert_eq!(args[0], "-u");
        assert_eq!(args[1], "root");
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

    #[test]
    fn test_nice_value_preserved() {
        let r = Runnable::builder()
            .name("test".to_string())
            .commands(vec!["echo hello".to_string()])
            .nice(Some(-5))
            .build();
        assert_eq!(r.nice, Some(-5));
    }

    #[test]
    fn test_nice_value_none_by_default() {
        let r = Runnable::builder().name("test".to_string()).commands(vec!["echo hello".to_string()]).build();
        assert_eq!(r.nice, None);
    }

    #[test]
    fn test_shell_nice_when_user_set() {
        let r = Runnable::builder()
            .name("test".to_string())
            .commands(vec!["echo hi".to_string()])
            .user(Some("root".to_string()))
            .nice(Some(10))
            .build();
        // With user set, shell method is auto-selected → nice -n appears in command
        let (prog, args) = args_list(&r);
        assert_eq!(prog, "sudo");
        assert!(args.last().unwrap().starts_with("nice -n 10"));
    }

    #[test]
    fn test_libc_nice_when_no_user() {
        let r =
            Runnable::builder().name("test".to_string()).commands(vec!["echo hi".to_string()]).nice(Some(10)).build();
        // Without user, libc method is auto-selected → nice -n NOT in command
        let (prog, args) = args_list(&r);
        assert_eq!(prog, "sh");
        assert_eq!(args.last().unwrap(), "echo hi");
    }

    #[tokio::test]
    async fn test_run_foreground_nice_no_user() {
        // Libc method (no user)
        let r = Runnable::builder()
            .name("test".to_string())
            .commands(vec!["echo hello".to_string()])
            .nice(Some(10))
            .build();
        let result = super::run_foreground(&r).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires sudo without password prompt"]
    async fn test_run_foreground_nice_with_user() {
        // Shell method (with user)
        let r = Runnable::builder()
            .name("test".to_string())
            .commands(vec!["echo hello".to_string()])
            .user(Some("root".to_string()))
            .nice(Some(10))
            .build();
        let result = super::run_foreground(&r).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_background_nice_no_user() {
        let r = Runnable::builder()
            .name("bg".to_string())
            .commands(vec!["echo background_nice".to_string()])
            .nice(Some(5))
            .build();
        let mut child = super::run_background(&r).await.unwrap();
        let status = child.wait().await.unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    #[ignore = "requires sudo without password prompt"]
    async fn test_run_background_nice_with_user() {
        let r = Runnable::builder()
            .name("bg".to_string())
            .commands(vec!["echo background_nice".to_string()])
            .user(Some("root".to_string()))
            .nice(Some(5))
            .build();
        let mut child = super::run_background(&r).await.unwrap();
        let status = child.wait().await.unwrap();
        assert!(status.success());
    }
}
