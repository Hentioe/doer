// 扩展 kdl 库
mod kdl_ext;
// 渲染命令、cwd 和环境变量值所用的模板
mod template;
// 一些解析辅助函数
mod helper;

use crate::prelude::*;
use doer_spec::{EnvVar, Runnable, StdIo as SpecStdIo};
use helper::*;
use kdl::{KdlDocument, KdlNode};
use kdl_ext::*;
use std::collections::{HashMap, HashSet};
use template::*;

#[derive(Debug)]
pub struct Config {
    pub tasks: Vec<Task>,
}

#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub commands: Vec<String>,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env_vars: Vec<EnvVar>,
    pub opts: Vec<Opt>,
    pub deps: Vec<Dep>,
    pub user: Option<String>,
    pub stdin: Option<String>,
    pub stderr: Option<String>,
    pub stdout: Option<String>,
}

#[derive(Debug)]
pub struct Opt {
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub struct Dep {
    pub name: String,
    pub args: Vec<String>,
    pub opts: Vec<Opt>,
    pub background: bool,
}

impl Task {
    fn substitution_context(&self, opt_overrides: &HashMap<String, String>) -> HashMap<String, String> {
        let mut ctx = HashMap::new();
        for (i, arg) in self.args.iter().enumerate() {
            ctx.insert(arg.clone(), format!("{{{i}}}"));
        }
        for opt in &self.opts {
            let value = opt_overrides.get(&opt.name).unwrap_or(&opt.value);
            ctx.insert(opt.name.clone(), value.clone());
        }
        ctx
    }

    pub fn command_template(&self, opt_overrides: &HashMap<String, String>) -> Result<Vec<String>> {
        self.commands.iter().map(|cmd| cmd.resolve_template(&self.substitution_context(opt_overrides))).collect()
    }

    pub fn build_commands(&self, args: &[String], opt_overrides: &HashMap<String, String>) -> Result<Vec<String>> {
        self.commands.iter().map(|cmd| cmd.resolve_both(&self.substitution_context(opt_overrides), args)).collect()
    }

    pub fn cwd_template(&self, opt_overrides: &HashMap<String, String>) -> Result<Option<String>> {
        self.cwd.as_ref().map(|cwd| cwd.resolve_template(&self.substitution_context(opt_overrides))).transpose()
    }

    pub fn build_cwd(&self, args: &[String], opt_overrides: &HashMap<String, String>) -> Result<Option<String>> {
        let ctx = self.substitution_context(opt_overrides);
        self.cwd.as_ref().map(|cwd| cwd.resolve_template(&ctx)?.resolve_args(args, "cwd template")).transpose()
    }

    pub fn env_vars_template(&self, opt_overrides: &HashMap<String, String>) -> Result<HashSet<EnvVar>> {
        let ctx = self.substitution_context(opt_overrides);
        self.env_vars.iter().map(|ev| ev.resolve_template(&ctx)).collect()
    }

    pub fn build_env_vars(&self, args: &[String], opt_overrides: &HashMap<String, String>) -> Result<HashSet<EnvVar>> {
        let ctx = self.substitution_context(opt_overrides);
        self.env_vars
            .iter()
            .map(|ev| {
                let label = format!("env var '{}'", ev.name);
                ev.resolve_template(&ctx)?.resolve_args(args, &label)
            })
            .collect()
    }

    pub fn build_stdin(&self, args: &[String], opt_overrides: &HashMap<String, String>) -> Result<SpecStdIo> {
        self.build_stdio_field(&self.stdin, args, opt_overrides, "stdin")
    }

    pub fn build_stdout(&self, args: &[String], opt_overrides: &HashMap<String, String>) -> Result<SpecStdIo> {
        self.build_stdio_field(&self.stdout, args, opt_overrides, "stdout")
    }

    pub fn build_stderr(&self, args: &[String], opt_overrides: &HashMap<String, String>) -> Result<SpecStdIo> {
        self.build_stdio_field(&self.stderr, args, opt_overrides, "stderr")
    }

    fn build_stdio_field(
        &self,
        field: &Option<String>,
        args: &[String],
        opt_overrides: &HashMap<String, String>,
        label: &str,
    ) -> Result<SpecStdIo> {
        let Some(raw) = field else {
            return Ok(SpecStdIo::default());
        };
        let ctx = self.substitution_context(opt_overrides);
        let resolved = raw.resolve_template(&ctx)?.resolve_args(args, &format!("{label} template"))?;
        SpecStdIo::try_from(resolved.as_str()).map_err(|e| anyhow::anyhow!("{}", e)).with_context(|| {
            format!(
                "task '{}': invalid {label} value after resolution: '{resolved}', possible values: [{}]",
                self.name,
                SpecStdIo::valid_string_values().join(", ")
            )
        })
    }

    pub fn build_dep(
        &self,
        dep: &Dep,
        args: &[String],
        opt_overrides: &HashMap<String, String>,
    ) -> Result<DepResolution> {
        let ctx = self.substitution_context(opt_overrides);
        let resolved_args: Vec<String> =
            dep.args.iter().map(|arg| arg.resolve_both(&ctx, args)).collect::<Result<Vec<_>>>()?;
        let resolved_opts: HashMap<String, String> = dep
            .opts
            .iter()
            .map(|opt| {
                let resolved_value = opt.value.resolve_both(&ctx, args)?;
                Ok((opt.name.clone(), resolved_value))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        Ok(DepResolution {
            name: dep.name.clone(),
            args: resolved_args,
            opt_overrides: resolved_opts,
            background: dep.background,
        })
    }
}

#[derive(Debug)]
pub struct TaskCall<'a> {
    pub task: &'a Task,
    pub args: Vec<String>,
    pub opt_overrides: HashMap<String, String>,
    pub background: bool,
}

pub struct DepResolution {
    pub name: String,
    pub args: Vec<String>,
    pub opt_overrides: HashMap<String, String>,
    pub background: bool,
}

impl Config {
    pub fn find_task(&self, name: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.name == name)
    }

    pub fn build_task_with_deps<'a>(
        &'a self,
        task_name: &str,
        args: &[String],
        opt_overrides: &HashMap<String, String>,
    ) -> Result<Vec<TaskCall<'a>>> {
        let mut resolved = Vec::new();
        let mut resolved_set = HashSet::new();
        let mut resolving = HashSet::new();
        self.build_task_with_deps_inner(
            task_name,
            args.to_vec(),
            opt_overrides.clone(),
            false,
            &mut resolved,
            &mut resolved_set,
            &mut resolving,
        )?;
        Ok(resolved)
    }

    // todo: 移除这个 allow
    #[allow(clippy::too_many_arguments)]
    fn build_task_with_deps_inner<'a>(
        &'a self,
        task_name: &str,
        args: Vec<String>,
        opt_overrides: HashMap<String, String>,
        background: bool,
        resolved: &mut Vec<TaskCall<'a>>,
        resolved_set: &mut HashSet<String>,
        resolving: &mut HashSet<String>,
    ) -> Result<()> {
        if resolved_set.contains(task_name) {
            return Ok(());
        }
        ensure!(
            !resolving.contains(task_name),
            "circular dependency detected: '{task_name}'"
        );

        resolving.insert(task_name.to_string());

        let task = self.find_task(task_name).with_context(|| format!("task '{task_name}' not found"))?;

        for dep in &task.deps {
            let r = task.build_dep(dep, &args, &opt_overrides)?;
            self.build_task_with_deps_inner(
                &r.name,
                r.args,
                r.opt_overrides,
                r.background,
                resolved,
                resolved_set,
                resolving,
            )?;
        }

        resolved.push(TaskCall {
            task,
            args,
            opt_overrides,
            background,
        });
        resolving.remove(task_name);
        resolved_set.insert(task_name.to_string());

        Ok(())
    }

    pub fn build_all(
        &self,
        task_name: &str,
        args: &[String],
        opt_overrides: &HashMap<String, String>,
    ) -> Result<Vec<Runnable>> {
        self.build_task_with_deps(task_name, args, opt_overrides)?
            .iter()
            .map(|call| {
                Ok(Runnable {
                    name: call.task.name.clone(),
                    commands: call.task.build_commands(&call.args, &call.opt_overrides)?,
                    cwd: call.task.build_cwd(&call.args, &call.opt_overrides)?,
                    env_vars: call.task.build_env_vars(&call.args, &call.opt_overrides)?,
                    user: call.task.user.clone(),
                    stdin: call.task.build_stdin(&call.args, &call.opt_overrides)?,
                    stdout: call.task.build_stdout(&call.args, &call.opt_overrides)?,
                    stderr: call.task.build_stderr(&call.args, &call.opt_overrides)?,
                    background: call.background,
                })
            })
            .collect()
    }

    pub fn load_from_kdl_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("failed to read config file")?;
        Self::from_kdl_str(&content)
    }

    pub fn from_kdl_str(content: &str) -> Result<Self> {
        let doc: KdlDocument = content.parse().context("failed to parse KDL config")?;
        let tasks_node = doc.get("tasks").context("missing 'tasks' node in config")?;
        let children = tasks_node.children().context("'tasks' node has no children block")?;

        let mut tasks = Vec::new();

        for node in children.nodes() {
            let name = node.name().value().to_string();

            let commands = parse_commands(node, &name)?;
            let args = parse_args(node, &name)?;
            let opts = parse_opts(node, &name)?;
            let deps = parse_deps(node, &name)?;

            if commands.is_empty() && deps.is_empty() {
                bail!("task '{}' has no command and no dependencies", name);
            }

            let user = parse_optional_string(node, &name, "user")?;
            let cwd = parse_optional_string(node, &name, "cwd")?;
            let env_vars = parse_env_vars(node, &name)?;
            let (stdin, stdout, stderr) = parse_stdio(node, &name)?;

            tasks.push(Task {
                name,
                commands,
                args,
                cwd,
                env_vars,
                opts,
                deps,
                user,
                stdin,
                stdout,
                stderr,
            });
        }

        Ok(Config { tasks })
    }
}

// ---- commands ----

pub fn parse_commands(node: &KdlNode, task_name: &str) -> Result<Vec<String>> {
    if let Some(entry) = node.first_entry() {
        ensure_entries_count(node, 1, "command").with_context(|| format!("task '{}'", task_name))?;
        entry
            .string_value()
            .with_context(|| format!("task '{}': command is not a string", task_name))
            .map(|s| vec![s.to_string()])
    } else if let Some(children) = node.children() {
        let dash_nodes = children.nodes_by_name("-");
        if dash_nodes.is_empty() {
            return Ok(Vec::new());
        }
        dash_nodes
            .iter()
            .map(|dash| {
                ensure_entries_count(dash, 1, "'-'").with_context(|| format!("task '{}'", task_name))?;
                dash.first_string()
                    .with_context(|| format!("task '{}': '-' has no command", task_name))
                    .map(|s| s.to_string())
            })
            .collect()
    } else {
        Ok(Vec::new())
    }
}

// ---- args ----

pub fn parse_args(node: &KdlNode, task_name: &str) -> Result<Vec<String>> {
    let Some(children) = node.children() else {
        return Ok(Vec::new());
    };
    children.nodes_by_name("arg").iter().map(|n| parse_arg(n, task_name)).collect()
}

pub fn parse_arg(node: &KdlNode, task_name: &str) -> Result<String> {
    ensure_entries_count(node, 1, "arg").with_context(|| format!("task '{}'", task_name))?;
    node.first_string()
        .with_context(|| format!("task '{}': arg value is not a string", task_name))
        .map(|s| s.to_string())
}

// ---- opts ----

pub fn parse_opts(node: &KdlNode, task_name: &str) -> Result<Vec<Opt>> {
    let Some(children) = node.children() else {
        return Ok(Vec::new());
    };
    children.nodes_by_name("opt").iter().map(|n| parse_opt(n, task_name)).collect()
}

pub fn parse_opt(node: &KdlNode, task_name: &str) -> Result<Opt> {
    ensure_entries_count(node, 1, "opt").with_context(|| format!("task '{}'", task_name))?;
    let entry = node.first_entry().with_context(|| format!("task '{}': opt has no entry", task_name))?;
    let name = entry.key().with_context(|| format!("task '{}': opt has no key", task_name))?.to_string();
    let value = entry_value_to_string(entry.value())
        .with_context(|| format!("task '{}': opt value is not a string or number", task_name))?;
    Ok(Opt { name, value })
}

fn entry_value_to_string(val: &kdl::KdlValue) -> Option<String> {
    if let Some(s) = val.as_string() {
        return Some(s.to_string());
    }
    if let Some(i) = val.as_integer() {
        return Some(i.to_string());
    }
    if let Some(f) = val.as_float() {
        return Some(f.to_string());
    }
    None
}

// ---- deps ----

pub fn parse_deps(node: &KdlNode, task_name: &str) -> Result<Vec<Dep>> {
    let Some(children) = node.children() else {
        return Ok(Vec::new());
    };
    children
        .nodes_by_name("dep")
        .iter()
        .map(|n| parse_dep(n))
        .collect::<Result<Vec<_>>>()
        .with_context(|| format!("task '{}'", task_name))
}

pub fn parse_dep(node: &KdlNode) -> Result<Dep> {
    ensure_entries_count(node, 1, "dep")?;
    let name = node.first_string().context("dep value is not a string")?.to_string();

    let args = match node.children() {
        Some(children) => children.nodes_by_name("arg").iter().map(|n| parse_dep_arg(n, &name)).collect(),
        None => Ok(Vec::new()),
    }?;
    let opts = match node.children() {
        Some(children) => children.nodes_by_name("opt").iter().map(|n| parse_opt(n, &name)).collect(),
        None => Ok(Vec::new()),
    }?;
    let background = parse_dep_background(node)?;

    Ok(Dep {
        name,
        args,
        opts,
        background,
    })
}

fn parse_dep_background(node: &KdlNode) -> Result<bool> {
    let Some(children) = node.children() else {
        return Ok(false);
    };
    let bg_nodes = children.nodes_by_name("background");
    if bg_nodes.is_empty() {
        return Ok(false);
    }
    ensure!(
        bg_nodes.len() == 1,
        "dep: expected at most 1 background node, got {}",
        bg_nodes.len()
    );
    let bg = bg_nodes[0];
    match bg.entries().len() {
        0 => Ok(true),
        1 => {
            let val = bg.entries().first().unwrap().value();
            val.as_bool().with_context(|| format!("dep background value is not a boolean: {val:?}"))
        }
        _ => bail!("dep: background node must have at most 1 entry"),
    }
}

pub fn parse_dep_arg(node: &KdlNode, dep_name: &str) -> Result<String> {
    ensure_entries_count(node, 1, "dep arg").with_context(|| format!("dep '{}'", dep_name))?;
    node.first_string().with_context(|| format!("dep '{}': arg value is not a string", dep_name)).map(|s| s.to_string())
}

// ---- user / cwd ----

pub fn parse_optional_string(node: &KdlNode, task_name: &str, field: &str) -> Result<Option<String>> {
    let Some(children) = node.children() else {
        return Ok(None);
    };
    let nodes = children.nodes_by_name(field);
    ensure!(
        nodes.len() <= 1,
        "task '{}': expected at most 1 {} node, got {}",
        task_name,
        field,
        nodes.len()
    );
    match nodes.first() {
        Some(n) => {
            ensure_entries_count(n, 1, field).with_context(|| format!("task '{}'", task_name))?;
            n.first_string()
                .with_context(|| format!("task '{}': {} value is not a string", task_name, field))
                .map(|s| Some(s.to_string()))
        }
        None => Ok(None),
    }
}

// ---- env_vars ----

pub fn parse_env_vars(node: &KdlNode, task_name: &str) -> Result<Vec<EnvVar>> {
    let Some(children) = node.children() else {
        return Ok(Vec::new());
    };
    let env_nodes = children.nodes_by_name("env");
    ensure!(
        env_nodes.len() <= 1,
        "task '{}': expected at most 1 env node, got {}",
        task_name,
        env_nodes.len()
    );
    let Some(env_node) = env_nodes.first() else {
        return Ok(Vec::new());
    };
    let Some(env_children) = env_node.children() else {
        return Ok(Vec::new());
    };
    env_children.nodes().iter().map(|n| parse_env_var(n, task_name)).collect()
}

pub fn parse_env_var(node: &KdlNode, task_name: &str) -> Result<EnvVar> {
    ensure_entries_count(node, 1, "env var").with_context(|| format!("task '{}'", task_name))?;
    let name = node.name().value().to_string();
    let entry = node.first_entry().with_context(|| format!("task '{}': env var has no entry", task_name))?;
    let value = entry_value_to_string(entry.value())
        .with_context(|| format!("task '{}': env var value is not a string or number", task_name))?;
    Ok(EnvVar { name, value })
}

// ---- stdio ----

pub fn parse_stdio(node: &KdlNode, task_name: &str) -> Result<(Option<String>, Option<String>, Option<String>)> {
    let stdin = parse_optional_stdio(node, task_name, "stdin")?;
    let stdout = parse_optional_stdio(node, task_name, "stdout")?;
    let stderr = parse_optional_stdio(node, task_name, "stderr")?;
    Ok((stdin, stdout, stderr))
}

fn parse_optional_stdio(node: &KdlNode, task_name: &str, field: &str) -> Result<Option<String>> {
    let Some(children) = node.children() else {
        return Ok(None);
    };
    let nodes = children.nodes_by_name(field);
    ensure!(
        nodes.len() <= 1,
        "task '{task_name}': expected at most 1 {field} node, got {}",
        nodes.len()
    );
    match nodes.first() {
        Some(n) => {
            ensure_entries_count(n, 1, field).with_context(|| format!("task '{}'", task_name))?;
            n.first_string()
                .with_context(|| format!("task '{}': {} value is not a string", task_name, field))
                .map(|s| Some(s.to_string()))
        }
        None => Ok(None),
    }
}
