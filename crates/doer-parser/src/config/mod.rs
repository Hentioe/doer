// 扩展 kdl 库
mod kdl_ext;
// 渲染命令、cwd 和环境变量值所用的模板
mod template;
// 一些解析辅助函数
mod helper;

use crate::prelude::*;
use anyhow::anyhow;
use doer_spec::{EnvVar, NICE_MAX, NICE_MIN, Runnable, SpecIo, VALID_STDIO_VALUES};
use helper::*;
use kdl::{KdlDocument, KdlNode};
use kdl_ext::*;
use std::collections::{HashMap, HashSet};
use template::*;
use typed_builder::TypedBuilder;

#[derive(Debug)]
pub struct Config {
    pub tasks: Vec<Task>,
    pub git_hooks: Option<bool>,
}

#[derive(Debug, TypedBuilder)]
pub struct Task {
    pub name: String,
    pub commands: Vec<String>,
    #[builder(default)]
    pub args: Vec<String>,
    #[builder(default)]
    pub cwd: Option<String>,
    #[builder(default)]
    pub env_vars: Vec<EnvVar>,
    #[builder(default)]
    pub opts: Vec<Opt>,
    #[builder(default)]
    pub deps: Vec<Dep>,
    #[builder(default)]
    pub user: Option<String>,
    #[builder(default)]
    pub stdin: Option<String>,
    #[builder(default)]
    pub stderr: Option<String>,
    #[builder(default)]
    pub stdout: Option<String>,
    #[builder(default)]
    pub nice: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptValue {
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct Opt {
    pub name: String,
    pub value: OptValue,
}

#[derive(Debug)]
pub struct Dep {
    pub name: String,
    pub args: Vec<String>,
    pub opts: Vec<Opt>,
    pub background: bool,
    pub stdin: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub nice: Option<i32>,
}

impl Task {
    fn substitution_context(&self, opt_overrides: &HashMap<String, OptValue>) -> HashMap<String, String> {
        let mut ctx = HashMap::new();
        for (i, arg) in self.args.iter().enumerate() {
            ctx.insert(arg.clone(), format!("{{{i}}}"));
        }
        // 首先插入所有 opt 的原始字符串值（无论是直接的字符串还是布尔值），以便后续解析时可以引用
        for opt in &self.opts {
            let value = match opt_overrides.get(&opt.name).unwrap_or(&opt.value) {
                OptValue::String(s) => s.clone(),
                OptValue::Bool(b) => b.to_string(),
            };
            ctx.insert(opt.name.clone(), value);
        }
        // 第二轮：解析布尔标志 — 任何布尔选项（或其默认值引用布尔值的选项）在为 true 时发出选项自身的名称，为 false 时发出空字符串。
        for opt in &self.opts {
            match &opt.value {
                OptValue::Bool(_) => {
                    let enabled = matches!(opt_overrides.get(&opt.name).unwrap_or(&opt.value), OptValue::Bool(true));
                    let flag = if enabled {
                        opt.name.clone()
                    } else {
                        FLAG_HIDDEN.to_string()
                    };
                    ctx.insert(opt.name.clone(), flag);
                }
                OptValue::String(s) => {
                    if let Some(ref_name) = s.strip_prefix('{').and_then(|s| s.strip_suffix('}'))
                        && self.opts.iter().any(|o| matches!(&o.value, OptValue::Bool(_)) && o.name == ref_name)
                    {
                        let enabled = if let Some(override_val) = opt_overrides.get(&opt.name) {
                            matches!(override_val, OptValue::Bool(true))
                        } else if let Some(ref_opt) = self.opts.iter().find(|o| o.name == ref_name) {
                            matches!(
                                opt_overrides.get(ref_name).unwrap_or(&ref_opt.value),
                                OptValue::Bool(true)
                            )
                        } else {
                            false
                        };
                        let flag = if enabled {
                            opt.name.clone()
                        } else {
                            FLAG_HIDDEN.to_string()
                        };
                        ctx.insert(opt.name.clone(), flag);
                    }
                }
            }
        }
        ctx
    }

    pub fn command_template(&self, opt_overrides: &HashMap<String, OptValue>) -> Result<Vec<String>> {
        self.commands.iter().map(|cmd| cmd.resolve_template(&self.substitution_context(opt_overrides))).collect()
    }

    pub fn build_commands(&self, args: &[String], opt_overrides: &HashMap<String, OptValue>) -> Result<Vec<String>> {
        self.commands.iter().map(|cmd| cmd.resolve_both(&self.substitution_context(opt_overrides), args)).collect()
    }

    pub fn cwd_template(&self, opt_overrides: &HashMap<String, OptValue>) -> Result<Option<String>> {
        self.cwd.as_ref().map(|cwd| cwd.resolve_template(&self.substitution_context(opt_overrides))).transpose()
    }

    pub fn build_cwd(&self, args: &[String], opt_overrides: &HashMap<String, OptValue>) -> Result<Option<String>> {
        let ctx = self.substitution_context(opt_overrides);
        self.cwd.as_ref().map(|cwd| cwd.resolve_template(&ctx)?.resolve_args(args, "cwd template")).transpose()
    }

    pub fn env_vars_template(&self, opt_overrides: &HashMap<String, OptValue>) -> Result<HashSet<EnvVar>> {
        let ctx = self.substitution_context(opt_overrides);
        self.env_vars.iter().map(|ev| ev.resolve_template(&ctx)).collect()
    }

    pub fn build_env_vars(
        &self,
        args: &[String],
        opt_overrides: &HashMap<String, OptValue>,
    ) -> Result<HashSet<EnvVar>> {
        let ctx = self.substitution_context(opt_overrides);
        self.env_vars
            .iter()
            .map(|ev| {
                let label = format!("env var '{}'", ev.name);
                ev.resolve_template(&ctx)?.resolve_args(args, &label)
            })
            .collect()
    }

    pub fn build_stdin(&self, args: &[String], opt_overrides: &HashMap<String, OptValue>) -> Result<SpecIo> {
        self.build_stdio_field(&self.stdin, args, opt_overrides, "stdin")
    }

    pub fn build_stdout(&self, args: &[String], opt_overrides: &HashMap<String, OptValue>) -> Result<SpecIo> {
        self.build_stdio_field(&self.stdout, args, opt_overrides, "stdout")
    }

    pub fn build_stderr(&self, args: &[String], opt_overrides: &HashMap<String, OptValue>) -> Result<SpecIo> {
        self.build_stdio_field(&self.stderr, args, opt_overrides, "stderr")
    }

    fn build_stdio_field(
        &self,
        field: &Option<String>,
        args: &[String],
        opt_overrides: &HashMap<String, OptValue>,
        label: &str,
    ) -> Result<SpecIo> {
        let Some(raw) = field else {
            return Ok(SpecIo::default());
        };
        let ctx = self.substitution_context(opt_overrides);
        let resolved = raw.resolve_template(&ctx)?.resolve_args(args, &format!("{label} template"))?;
        SpecIo::try_from(resolved.as_str()).map_err(|e| anyhow::anyhow!("{}", e)).with_context(|| {
            format!(
                "task '{}': invalid {label} value after resolution: '{resolved}', possible values: [{}]",
                self.name,
                VALID_STDIO_VALUES.join(", ")
            )
        })
    }

    pub fn build_dep(
        &self,
        dep: &Dep,
        args: &[String],
        opt_overrides: &HashMap<String, OptValue>,
    ) -> Result<DepResolution> {
        let ctx = self.substitution_context(opt_overrides);
        let resolved_args: Vec<String> =
            dep.args.iter().map(|arg| arg.resolve_both(&ctx, args)).collect::<Result<Vec<_>>>()?;
        let resolved_opts: HashMap<String, OptValue> = dep
            .opts
            .iter()
            .map(|opt| {
                let resolved_value = match &opt.value {
                    OptValue::String(s) => OptValue::String(s.resolve_both(&ctx, args)?),
                    OptValue::Bool(b) => OptValue::Bool(*b),
                };
                Ok((opt.name.clone(), resolved_value))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        let resolve_stdio = |v: &Option<String>| -> Result<Option<String>> {
            v.as_ref().map(|s| s.resolve_both(&ctx, args)).transpose()
        };
        Ok(DepResolution {
            name: dep.name.clone(),
            args: resolved_args,
            opt_overrides: resolved_opts,
            background: dep.background,
            stdin: resolve_stdio(&dep.stdin)?,
            stdout: resolve_stdio(&dep.stdout)?,
            stderr: resolve_stdio(&dep.stderr)?,
            nice: dep.nice,
        })
    }
}

#[derive(Debug)]
pub struct TaskCall<'a> {
    pub task: &'a Task,
    pub args: Vec<String>,
    pub opt_overrides: HashMap<String, OptValue>,
    pub background: bool,
    pub stdin: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub nice: Option<i32>,
}

pub struct DepResolution {
    pub name: String,
    pub args: Vec<String>,
    pub opt_overrides: HashMap<String, OptValue>,
    pub background: bool,
    pub stdin: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub nice: Option<i32>,
}

impl Config {
    pub fn find_task(&self, name: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.name == name)
    }

    pub fn build_task_with_deps<'a>(
        &'a self,
        task_name: &str,
        args: &[String],
        opt_overrides: &HashMap<String, OptValue>,
    ) -> Result<Vec<TaskCall<'a>>> {
        let mut resolved = Vec::new();
        let mut resolved_set = HashSet::new();
        let mut resolving = HashSet::new();
        self.build_task_with_deps_inner(
            task_name,
            args.to_vec(),
            opt_overrides.clone(),
            false,
            None,
            None,
            None,
            None,
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
        opt_overrides: HashMap<String, OptValue>,
        background: bool,
        nice: Option<i32>,
        stdin: Option<String>,
        stdout: Option<String>,
        stderr: Option<String>,
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
                r.nice,
                r.stdin,
                r.stdout,
                r.stderr,
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
            nice: nice.or(task.nice),
            stdin,
            stdout,
            stderr,
        });
        resolving.remove(task_name);
        resolved_set.insert(task_name.to_string());

        Ok(())
    }

    pub fn build_all(
        &self,
        task_name: &str,
        args: &[String],
        opt_overrides: &HashMap<String, OptValue>,
    ) -> Result<Vec<Runnable>> {
        self.build_task_with_deps(task_name, args, opt_overrides)?
            .iter()
            .map(|call| {
                let stdin = match &call.stdin {
                    Some(val) => parse_spec_stdio(val, "stdin", &call.task.name)?,
                    None => call.task.build_stdin(&call.args, &call.opt_overrides)?,
                };
                let stdout = match &call.stdout {
                    Some(val) => parse_spec_stdio(val, "stdout", &call.task.name)?,
                    None => call.task.build_stdout(&call.args, &call.opt_overrides)?,
                };
                let stderr = match &call.stderr {
                    Some(val) => parse_spec_stdio(val, "stderr", &call.task.name)?,
                    None => call.task.build_stderr(&call.args, &call.opt_overrides)?,
                };
                let runnable = Runnable::builder()
                    .name(call.task.name.clone())
                    .commands(call.task.build_commands(&call.args, &call.opt_overrides)?)
                    .cwd(call.task.build_cwd(&call.args, &call.opt_overrides)?)
                    .env_vars(call.task.build_env_vars(&call.args, &call.opt_overrides)?)
                    .user(call.task.user.clone())
                    .stdin(stdin)
                    .stdout(stdout)
                    .stderr(stderr)
                    .background(call.background)
                    .nice(call.nice)
                    .build();

                Ok(runnable)
            })
            .collect()
    }

    pub fn load_from_kdl_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("failed to read config file")?;
        Self::from_kdl_str(&content)
    }

    pub fn from_kdl_str(content: &str) -> Result<Self> {
        // todo； 获取解析失败的具体信息
        let doc: KdlDocument = content.parse().map_err(|_| anyhow!("failed to parse KDL content"))?;
        let tasks_node = doc.get("tasks").context("missing 'tasks' node in config")?;
        let children = tasks_node.children().context("'tasks' node has no children block")?;

        let git_hooks = doc
            .nodes()
            .iter()
            .find(|n| n.name().value() == "git-hooks")
            .map(|n| {
                let entries = n.entries();
                ensure!(entries.len() == 1, "git-hooks: expected 1 entry, got {}", entries.len());
                let val = entries.first().context("git-hooks: expected a boolean value")?.value();
                val.as_bool().with_context(|| format!("git-hooks value is not a boolean: {val:?}"))
            })
            .transpose()?;

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
            let nice = parse_optional_integer(node, &name, "nice")?;

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
                nice,
            });
        }

        Ok(Config { tasks, git_hooks })
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
                ensure_entries_count(dash, 1, "'-'").with_context(|| format!("task: {}", task_name))?;
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
    let value = entry_opt_value(entry.value())
        .with_context(|| format!("task '{}': opt value is not a string, number, or boolean", task_name))?;
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

fn entry_opt_value(val: &kdl::KdlValue) -> Option<OptValue> {
    if let Some(b) = val.as_bool() {
        return Some(OptValue::Bool(b));
    }
    entry_value_to_string(val).map(OptValue::String)
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
    let (stdin, stdout, stderr) = parse_dep_stdio(node, &name)?;
    let nice = parse_dep_nice(node, &name)?;

    Ok(Dep {
        name,
        args,
        opts,
        background,
        stdin,
        stdout,
        stderr,
        nice,
    })
}

fn parse_dep_stdio(node: &KdlNode, dep_name: &str) -> Result<(Option<String>, Option<String>, Option<String>)> {
    let Some(children) = node.children() else {
        return Ok((None, None, None));
    };
    let stdin = parse_dep_optional_stdio(children, dep_name, "stdin")?;
    let stdout = parse_dep_optional_stdio(children, dep_name, "stdout")?;
    let stderr = parse_dep_optional_stdio(children, dep_name, "stderr")?;
    Ok((stdin, stdout, stderr))
}

fn parse_dep_optional_stdio(children: &KdlDocument, dep_name: &str, field: &str) -> Result<Option<String>> {
    let nodes = children.nodes_by_name(field);
    ensure!(
        nodes.len() <= 1,
        "dep '{dep_name}': expected at most 1 {field} node, got {}",
        nodes.len()
    );
    match nodes.first() {
        Some(n) => {
            ensure_entries_count(n, 1, field).with_context(|| format!("dep '{}'", dep_name))?;
            n.first_string()
                .with_context(|| format!("dep '{}': {} value is not a string", dep_name, field))
                .map(|s| Some(s.to_string()))
        }
        None => Ok(None),
    }
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
            let val = bg.entries().first().context("dep background: expected a boolean value")?.value();
            val.as_bool().with_context(|| format!("dep background value is not a boolean: {val:?}"))
        }
        _ => bail!("dep: background node must have at most 1 entry"),
    }
}

fn parse_dep_nice(node: &KdlNode, dep_name: &str) -> Result<Option<i32>> {
    let Some(children) = node.children() else {
        return Ok(None);
    };
    let nice_nodes = children.nodes_by_name("nice");
    if nice_nodes.is_empty() {
        return Ok(None);
    }
    ensure!(
        nice_nodes.len() == 1,
        "dep '{dep_name}': expected at most 1 nice node, got {}",
        nice_nodes.len()
    );
    let nice = nice_nodes[0];
    ensure_entries_count(nice, 1, "nice").with_context(|| format!("dep '{dep_name}'"))?;
    let entry = nice.first_entry().context(format!("dep '{dep_name}': nice has no entry"))?;
    let raw = entry.value().as_integer().with_context(|| format!("dep '{dep_name}': nice value is not an integer"))?;
    let val = i32::try_from(raw)
        .map_err(|_| anyhow!("dep '{dep_name}': nice value {raw} is out of range (valid: {NICE_MIN}..={NICE_MAX})"))?;
    ensure!(
        (NICE_MIN..=NICE_MAX).contains(&val),
        "dep '{dep_name}': nice value {val} is out of range (valid: {NICE_MIN}..={NICE_MAX})"
    );
    Ok(Some(val))
}

pub fn parse_dep_arg(node: &KdlNode, dep_name: &str) -> Result<String> {
    ensure_entries_count(node, 1, "dep arg").with_context(|| format!("dep '{}'", dep_name))?;
    node.first_string().with_context(|| format!("dep '{}': arg value is not a string", dep_name)).map(|s| s.to_string())
}

// ---- user / cwd ----

pub fn parse_optional_integer(node: &KdlNode, task_name: &str, field: &str) -> Result<Option<i32>> {
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
            let entry = n.first_entry().context(format!("task '{}': {} has no entry", task_name, field))?;
            let raw = entry
                .value()
                .as_integer()
                .with_context(|| format!("task '{}': {} value is not an integer", task_name, field))?;
            let val = i32::try_from(raw).map_err(|_| {
                anyhow::anyhow!(
                    "task '{}': {} value {} is out of range (valid: {NICE_MIN}..={NICE_MAX})",
                    task_name,
                    field,
                    raw
                )
            })?;
            ensure!(
                (NICE_MIN..=NICE_MAX).contains(&val),
                "task '{}': {} value {} is out of range (valid: {NICE_MIN}..={NICE_MAX})",
                task_name,
                field,
                val
            );
            Ok(Some(val))
        }
        None => Ok(None),
    }
}

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

pub fn parse_spec_stdio(raw: &str, label: &str, task_name: &str) -> Result<SpecIo> {
    SpecIo::try_from(raw).map_err(|e| anyhow::anyhow!("{}", e)).with_context(|| {
        format!(
            "task '{task_name}': invalid {label} value: '{raw}', possible values: [{}]",
            VALID_STDIO_VALUES.join(", ")
        )
    })
}

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
