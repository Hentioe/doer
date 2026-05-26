use crate::prelude::*;
use doer_parser::Config;
use doer_spec::warn;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const HOOKS_DIR: &str = ".githooks";

/// Generate git hooks if conditions are met.
pub fn ensure_hooks(config: &Config) -> Result<()> {
    if !Path::new(".git").exists() {
        return Ok(());
    }

    if let Some(false) = config.git_hooks {
        return Ok(());
    }

    let mut generated = false;

    if let Some(task_name) = find_preferred_task(config, &["pre-commit", "precommit"]) {
        write_hook_script("pre-commit", &task_name)?;
        generated = true;
    }

    if let Some(task_name) = find_preferred_task(config, &["pre-push", "prepush"]) {
        write_hook_script("pre-push", &task_name)?;
        generated = true;
    }

    if generated {
        set_hooks_path()?;
    }

    Ok(())
}

fn find_preferred_task(config: &Config, candidates: &[&str]) -> Option<String> {
    candidates.iter().find(|&&name| config.tasks.iter().any(|t| t.name == name)).map(|&name| name.to_string())
}

fn write_hook_script(hook_name: &str, task_name: &str) -> Result<()> {
    let hooks_dir = Path::new(HOOKS_DIR);
    if !hooks_dir.exists() {
        std::fs::create_dir_all(hooks_dir).context("failed to create .githooks directory")?;
    }
    let script_path = hooks_dir.join(hook_name);

    if script_path.exists() {
        // 如果脚本存在，不覆盖它，避免破坏用户修改过的钩子
        return Ok(());
    }
    let content = format!("#!/bin/sh\ndoer {task_name}\n");

    std::fs::write(&script_path, &content).with_context(|| format!("failed to write hook script '{hook_name}'"))?;
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("failed to set permissions on '{hook_name}'"))?;

    warn!("created {HOOKS_DIR}/{hook_name}: 'doer {task_name}'");

    Ok(())
}

fn set_hooks_path() -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["config", "core.hooksPath", HOOKS_DIR])
        .output()
        .context("failed to run git config")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git config core.hooksPath failed: {stderr}");
    }

    Ok(())
}
