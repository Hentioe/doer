use doer_runner;
use doer_spec::{Runnable, StdIo as SpecStdIo};
use std::collections::HashSet;

fn simple_runnable(name: &str, command: &str) -> Runnable {
    Runnable {
        name: name.to_string(),
        commands: vec![command.to_string()],
        cwd: None,
        env_vars: HashSet::new(),
        user: None,
        background: false,
        stderr: SpecStdIo::Inherit,
        stdout: SpecStdIo::Inherit,
        stdin: SpecStdIo::Inherit,
    }
}

fn bg_runnable(name: &str, command: &str) -> Runnable {
    Runnable {
        name: name.to_string(),
        commands: vec![command.to_string()],
        cwd: None,
        env_vars: HashSet::new(),
        user: None,
        background: true,
        stderr: SpecStdIo::Inherit,
        stdout: SpecStdIo::Inherit,
        stdin: SpecStdIo::Inherit,
    }
}

#[tokio::test]
async fn test_run_foreground_success() {
    let r = simple_runnable("test", "echo hello");
    let result = doer_runner::run_foreground(&r).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_foreground_failure() {
    let r = simple_runnable("test", "exit 1");
    let result = doer_runner::run_foreground(&r).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_run_foreground_all() {
    let r1 = simple_runnable("first", "echo one");
    let r2 = simple_runnable("second", "echo two");
    let runnables = vec![r1, r2];
    let result = doer_runner::run_foreground_all(&runnables).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_foreground_all_stops_on_failure() {
    let r1 = simple_runnable("first", "exit 1");
    let r2 = simple_runnable("second", "echo should_not_run");
    let runnables = vec![r1, r2];
    let result = doer_runner::run_foreground_all(&runnables).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_run_background() {
    let r = simple_runnable("bg", "echo background_test");
    let child = doer_runner::run_background(&r).await.unwrap();
    let output = child.wait_with_output().await.unwrap();
    assert!(output.status.success());
}

#[tokio::test]
async fn test_run_background_all() {
    let r1 = simple_runnable("bg1", "echo one");
    let r2 = simple_runnable("bg2", "echo two");
    let children = doer_runner::run_background_all(&vec![r1, r2]).await.unwrap();
    for child in children {
        let output = child.wait_with_output().await.unwrap();
        assert!(output.status.success());
    }
}

#[tokio::test]
async fn test_run_all_simple() {
    let r1 = simple_runnable("first", "echo hello");
    let r2 = simple_runnable("second", "echo world");
    let result = doer_runner::run_all(&vec![r1, r2]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_all_mixed_fg_bg() {
    let r1 = bg_runnable("bg-dep", "echo background");
    let r2 = simple_runnable("main", "echo foreground");
    let result = doer_runner::run_all(&vec![r1, r2]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_all_bg_survives_main() {
    let r1 = bg_runnable("bg", "sleep 0.1 && echo bg_done");
    let r2 = simple_runnable("main", "echo main_done");
    let result = doer_runner::run_all(&vec![r1, r2]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_all_fg_fails_stops() {
    let r1 = simple_runnable("will-fail", "exit 1");
    let r2 = simple_runnable("never-runs", "echo nope");
    let result = doer_runner::run_all(&vec![r1, r2]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_run_background_multi_command() {
    let r = Runnable {
        name: "multi-bg".into(),
        commands: vec!["echo first".into(), "echo background_last".into()],
        cwd: None,
        env_vars: HashSet::new(),
        user: None,
        background: true,
        stderr: SpecStdIo::Inherit,
        stdout: SpecStdIo::Inherit,
        stdin: SpecStdIo::Inherit,
    };
    let mut child = doer_runner::run_background(&r).await.unwrap();
    let status = child.wait().await.unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn test_run_all_fg_multi_command() {
    let r = simple_runnable("multi-fg", "echo first");
    let r = Runnable {
        commands: vec!["echo first".into(), "echo second".into()],
        ..r
    };
    let result = doer_runner::run_foreground(&r).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_all_fg_multi_command_stops_on_failure() {
    let r = Runnable {
        name: "multi-fg".into(),
        commands: vec!["echo first".into(), "exit 1".into(), "echo nope".into()],
        cwd: None,
        env_vars: HashSet::new(),
        user: None,
        background: false,
        stderr: SpecStdIo::Inherit,
        stdout: SpecStdIo::Inherit,
        stdin: SpecStdIo::Inherit,
    };
    let result = doer_runner::run_foreground(&r).await;
    assert!(result.is_err());
}
