use crate::prelude::*;
use doer_parser::Config;
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

    if hooks_synced() {
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

fn hooks_synced() -> bool {
    std::process::Command::new("git")
        .args(["config", "core.hooksPath"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok()
            } else {
                None
            }
        })
        .is_some_and(|path| path.trim() == HOOKS_DIR)
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
    let content = format!("#!/bin/sh\ndoer {task_name}\n");

    std::fs::write(&script_path, &content).with_context(|| format!("failed to write hook script '{hook_name}'"))?;

    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("failed to set permissions on '{hook_name}'"))?;

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
