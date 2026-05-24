use doer_parser::config::{Config, Dep, Opt, Task, TaskCall};
use doer_spec::EnvVar;
use std::collections::{HashMap, HashSet};

fn task(name: &str, command: &str, args: Vec<&str>, opts: Vec<(&str, &str)>) -> Task {
    task_with_cwd(name, command, None, args, opts)
}

fn task_with_cwd(name: &str, command: &str, cwd: Option<&str>, args: Vec<&str>, opts: Vec<(&str, &str)>) -> Task {
    Task {
        name: name.to_string(),
        commands: vec![command.to_string()],
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: cwd.map(|s| s.to_string()),
        env_vars: vec![],
        opts: opts
            .iter()
            .map(|(k, v)| Opt {
                name: k.to_string(),
                value: v.to_string(),
            })
            .collect(),
        deps: vec![],
        user: None,
        stdin: None,
        stdout: None,
        stderr: None,
    }
}

fn task_with_env(
    name: &str,
    command: &str,
    env_vars: Vec<(&str, &str)>,
    args: Vec<&str>,
    opts: Vec<(&str, &str)>,
) -> Task {
    Task {
        name: name.to_string(),
        commands: vec![command.to_string()],
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: None,
        env_vars: env_vars
            .iter()
            .map(|(k, v)| EnvVar {
                name: k.to_string(),
                value: v.to_string(),
            })
            .collect(),
        opts: opts
            .iter()
            .map(|(k, v)| Opt {
                name: k.to_string(),
                value: v.to_string(),
            })
            .collect(),
        deps: vec![],
        user: None,
        stdin: None,
        stdout: None,
        stderr: None,
    }
}

fn task_with_deps(name: &str, command: &str, args: Vec<&str>, opts: Vec<(&str, &str)>, deps: Vec<Dep>) -> Task {
    Task {
        name: name.to_string(),
        commands: vec![command.to_string()],
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: None,
        env_vars: vec![],
        opts: opts
            .iter()
            .map(|(k, v)| Opt {
                name: k.to_string(),
                value: v.to_string(),
            })
            .collect(),
        deps,
        user: None,
        stdin: None,
        stdout: None,
        stderr: None,
    }
}

fn s(v: &str) -> String {
    v.to_string()
}

fn no_overrides() -> HashMap<String, String> {
    HashMap::new()
}

fn overrides(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn h(pairs: &[(&str, &str)]) -> HashSet<EnvVar> {
    pairs
        .iter()
        .map(|(k, v)| EnvVar {
            name: k.to_string(),
            value: v.to_string(),
        })
        .collect()
}

// ===================================================================
// command_template — no variables
// ===================================================================

#[test]
fn no_variables() {
    let t = task("build", "cargo build", vec![], vec![]);
    assert_eq!(t.command_template(&no_overrides()).unwrap(), vec!["cargo build"]);
}

// ===================================================================
// command_template — args only
// ===================================================================

#[test]
fn arg_substitution_single() {
    let t = task("run", "cargo run --bin {bin}", vec!["bin"], vec![]);
    assert_eq!(
        t.command_template(&no_overrides()).unwrap(),
        vec!["cargo run --bin {0}"]
    );
}

#[test]
fn arg_substitution_multiple() {
    let t = task("run", "cmd {a} {b} {c}", vec!["a", "b", "c"], vec![]);
    assert_eq!(t.command_template(&no_overrides()).unwrap(), vec!["cmd {0} {1} {2}"]);
}

#[test]
fn arg_referenced_multiple_times() {
    let t = task("echo", "{x} {x} {x}", vec!["x"], vec![]);
    assert_eq!(t.command_template(&no_overrides()).unwrap(), vec!["{0} {0} {0}"]);
}

// ===================================================================
// command_template — opts only
// ===================================================================

#[test]
fn opt_substitution_single() {
    let t = task("build", "cargo build --{mode}", vec![], vec![("mode", "debug")]);
    assert_eq!(
        t.command_template(&no_overrides()).unwrap(),
        vec!["cargo build --debug"]
    );
}

#[test]
fn opt_substitution_multiple() {
    let t = task("build", "{a}/{b}", vec![], vec![("a", "foo"), ("b", "bar")]);
    assert_eq!(t.command_template(&no_overrides()).unwrap(), vec!["foo/bar"]);
}

// ===================================================================
// command_template — opt overrides
// ===================================================================

#[test]
fn opt_override_single() {
    let t = task("build", "cargo build --{mode}", vec![], vec![("mode", "debug")]);
    assert_eq!(
        t.command_template(&overrides(&[("mode", "release")])).unwrap(),
        vec!["cargo build --release"]
    );
}

#[test]
fn opt_override_partial() {
    let t = task("build", "{a}/{b}", vec![], vec![("a", "foo"), ("b", "bar")]);
    assert_eq!(t.command_template(&overrides(&[("a", "X")])).unwrap(), vec!["X/bar"]);
}

#[test]
fn opt_override_unknown_ignored() {
    let t = task("build", "cargo build --{mode}", vec![], vec![("mode", "debug")]);
    assert_eq!(
        t.command_template(&overrides(&[("unknown", "X")])).unwrap(),
        vec!["cargo build --debug"]
    );
}

// ===================================================================
// command_template — mixed args and opts
// ===================================================================

#[test]
fn mixed_args_and_opts() {
    let t = task(
        "install",
        "install --bin {bin} --path {path}",
        vec!["path"],
        vec![("bin", "doer")],
    );
    assert_eq!(
        t.command_template(&no_overrides()).unwrap(),
        vec!["install --bin doer --path {0}"]
    );
}

// ===================================================================
// command_template — errors
// ===================================================================

#[test]
fn undefined_variable() {
    let t = task("run", "cmd {undefined}", vec![], vec![]);
    let err = t.command_template(&no_overrides()).unwrap_err();
    assert!(format!("{:#}", err).contains("undefined variable '{undefined}'"));
}

#[test]
fn unclosed_brace() {
    let t = task("run", "cmd {foo", vec!["foo"], vec![]);
    let err = t.command_template(&no_overrides()).unwrap_err();
    assert!(format!("{:#}", err).contains("unclosed '{'"));
}

#[test]
fn empty_placeholder() {
    let t = task("run", "cmd {}", vec![], vec![]);
    let err = t.command_template(&no_overrides()).unwrap_err();
    assert!(format!("{:#}", err).contains("empty placeholder '{}'"));
}

#[test]
fn unexpected_closing_brace() {
    let t = task("run", "cmd }", vec![], vec![]);
    let err = t.command_template(&no_overrides()).unwrap_err();
    assert!(format!("{:#}", err).contains("unexpected '}'"));
}

#[test]
fn no_substitution_context_unchanged() {
    let t = task("simple", "echo hello world", vec![], vec![]);
    assert_eq!(t.command_template(&no_overrides()).unwrap(), vec!["echo hello world"]);
}

// ===================================================================
// build_command
// ===================================================================

#[test]
fn build_no_args_no_opts() {
    let t = task("build", "cargo build", vec![], vec![]);
    assert_eq!(t.build_commands(&[], &no_overrides()).unwrap(), vec!["cargo build"]);
}

#[test]
fn build_single_arg() {
    let t = task("run", "cargo run --bin {bin}", vec!["bin"], vec![]);
    assert_eq!(
        t.build_commands(&[s("doer")], &no_overrides()).unwrap(),
        vec!["cargo run --bin doer"]
    );
}

#[test]
fn build_multiple_args() {
    let t = task("cmd", "cmd {a} {b} {c}", vec!["a", "b", "c"], vec![]);
    assert_eq!(
        t.build_commands(&[s("x"), s("y"), s("z")], &no_overrides()).unwrap(),
        vec!["cmd x y z"]
    );
}

#[test]
fn build_mixed_args_and_opts() {
    let t = task(
        "install",
        "install --bin {bin} --path {path}",
        vec!["path"],
        vec![("bin", "doer")],
    );
    assert_eq!(
        t.build_commands(&[s("/usr/local")], &no_overrides()).unwrap(),
        vec!["install --bin doer --path /usr/local"]
    );
}

#[test]
fn build_with_opt_override() {
    let t = task(
        "install",
        "install --bin {bin} --path {path}",
        vec!["path"],
        vec![("bin", "doer")],
    );
    assert_eq!(
        t.build_commands(&[s("/usr/local")], &overrides(&[("bin", "myapp")])).unwrap(),
        vec!["install --bin myapp --path /usr/local"]
    );
}

#[test]
fn build_arg_repeated() {
    let t = task("echo", "{x} and {x}", vec!["x"], vec![]);
    assert_eq!(t.build_commands(&[s("a")], &no_overrides()).unwrap(), vec!["a and a"]);
}

#[test]
fn build_missing_arg() {
    let t = task("cmd", "cmd {a}", vec!["a"], vec![]);
    let err = t.build_commands(&[], &no_overrides()).unwrap_err();
    assert!(format!("{:#}", err).contains("missing argument at position 0"));
}

#[test]
fn build_extra_args_ignored() {
    let t = task("cmd", "cmd {a}", vec!["a"], vec![]);
    assert_eq!(
        t.build_commands(&[s("x"), s("y")], &no_overrides()).unwrap(),
        vec!["cmd x"]
    );
}

#[test]
fn build_keeps_opts_without_args() {
    let t = task("build", "cargo build --{mode}", vec![], vec![("mode", "release")]);
    assert_eq!(
        t.build_commands(&[], &no_overrides()).unwrap(),
        vec!["cargo build --release"]
    );
}

#[test]
fn build_empty_command() {
    let t = task("empty", "", vec![], vec![]);
    assert_eq!(t.build_commands(&[], &no_overrides()).unwrap(), vec![""]);
}

// ===================================================================
// cwd_template
// ===================================================================

#[test]
fn cwd_template_none() {
    let t = task("build", "cargo build", vec![], vec![]);
    assert_eq!(t.cwd_template(&no_overrides()).unwrap(), None);
}

#[test]
fn cwd_template_no_variables() {
    let t = task_with_cwd("build", "cargo build", Some("/home/user"), vec![], vec![]);
    assert_eq!(t.cwd_template(&no_overrides()).unwrap(), Some("/home/user".into()));
}

#[test]
fn cwd_template_with_arg() {
    let t = task_with_cwd("build", "cmd", Some("target/{mode}"), vec!["mode"], vec![]);
    assert_eq!(t.cwd_template(&no_overrides()).unwrap(), Some("target/{0}".into()));
}

#[test]
fn cwd_template_with_opt() {
    let t = task_with_cwd("build", "cmd", Some("target/{mode}"), vec![], vec![("mode", "debug")]);
    assert_eq!(t.cwd_template(&no_overrides()).unwrap(), Some("target/debug".into()));
}

#[test]
fn cwd_template_opt_override() {
    let t = task_with_cwd("build", "cmd", Some("target/{mode}"), vec![], vec![("mode", "debug")]);
    assert_eq!(
        t.cwd_template(&overrides(&[("mode", "release")])).unwrap(),
        Some("target/release".into())
    );
}

#[test]
fn cwd_template_mixed() {
    let t = task_with_cwd(
        "build",
        "cmd",
        Some("{base}/{mode}/build"),
        vec!["base"],
        vec![("mode", "release")],
    );
    assert_eq!(
        t.cwd_template(&no_overrides()).unwrap(),
        Some("{0}/release/build".into())
    );
}

// ===================================================================
// build_cwd
// ===================================================================

#[test]
fn build_cwd_none() {
    let t = task("build", "cargo build", vec![], vec![]);
    assert_eq!(t.build_cwd(&[], &no_overrides()).unwrap(), None);
}

#[test]
fn build_cwd_no_variables() {
    let t = task_with_cwd("build", "cmd", Some("/home/user"), vec![], vec![]);
    assert_eq!(t.build_cwd(&[], &no_overrides()).unwrap(), Some("/home/user".into()));
}

#[test]
fn build_cwd_with_arg() {
    let t = task_with_cwd("build", "cmd", Some("target/{mode}"), vec!["mode"], vec![]);
    assert_eq!(
        t.build_cwd(&[s("release")], &no_overrides()).unwrap(),
        Some("target/release".into())
    );
}

#[test]
fn build_cwd_with_opt() {
    let t = task_with_cwd("build", "cmd", Some("target/{mode}"), vec![], vec![("mode", "debug")]);
    assert_eq!(t.build_cwd(&[], &no_overrides()).unwrap(), Some("target/debug".into()));
}

#[test]
fn build_cwd_with_opt_override() {
    let t = task_with_cwd("build", "cmd", Some("target/{mode}"), vec![], vec![("mode", "debug")]);
    assert_eq!(
        t.build_cwd(&[], &overrides(&[("mode", "release")])).unwrap(),
        Some("target/release".into())
    );
}

#[test]
fn build_cwd_mixed() {
    let t = task_with_cwd(
        "build",
        "cmd",
        Some("{base}/{mode}/build"),
        vec!["base"],
        vec![("mode", "release")],
    );
    assert_eq!(
        t.build_cwd(&[s("src")], &no_overrides()).unwrap(),
        Some("src/release/build".into())
    );
}

// ===================================================================
// env_vars_template
// ===================================================================

#[test]
fn env_vars_template_empty() {
    let t = task("build", "cargo build", vec![], vec![]);
    assert!(t.env_vars_template(&no_overrides()).unwrap().is_empty());
}

#[test]
fn env_vars_template_no_variables() {
    let t = task_with_env("run", "cmd", vec![("RUST_LOG", "debug")], vec![], vec![]);
    assert_eq!(
        t.env_vars_template(&no_overrides()).unwrap(),
        h(&[("RUST_LOG", "debug")])
    );
}

#[test]
fn env_vars_template_with_arg() {
    let t = task_with_env("run", "cmd", vec![("BIN", "{bin}")], vec!["bin"], vec![]);
    assert_eq!(t.env_vars_template(&no_overrides()).unwrap(), h(&[("BIN", "{0}")]));
}

#[test]
fn env_vars_template_with_opt() {
    let t = task_with_env(
        "run",
        "cmd",
        vec![("MODE", "{mode}")],
        vec![],
        vec![("mode", "release")],
    );
    assert_eq!(t.env_vars_template(&no_overrides()).unwrap(), h(&[("MODE", "release")]));
}

#[test]
fn env_vars_template_opt_override() {
    let t = task_with_env("run", "cmd", vec![("MODE", "{mode}")], vec![], vec![("mode", "debug")]);
    assert_eq!(
        t.env_vars_template(&overrides(&[("mode", "release")])).unwrap(),
        h(&[("MODE", "release")])
    );
}

#[test]
fn env_vars_template_multiple() {
    let t = task_with_env(
        "run",
        "cmd",
        vec![("A", "{x}"), ("B", "{y}")],
        vec!["x"],
        vec![("y", "bar")],
    );
    assert_eq!(
        t.env_vars_template(&no_overrides()).unwrap(),
        h(&[("A", "{0}"), ("B", "bar")])
    );
}

// ===================================================================
// build_env_vars
// ===================================================================

#[test]
fn build_env_vars_empty() {
    let t = task("build", "cargo build", vec![], vec![]);
    assert!(t.build_env_vars(&[], &no_overrides()).unwrap().is_empty());
}

#[test]
fn build_env_vars_no_variables() {
    let t = task_with_env("run", "cmd", vec![("RUST_LOG", "debug")], vec![], vec![]);
    assert_eq!(
        t.build_env_vars(&[], &no_overrides()).unwrap(),
        h(&[("RUST_LOG", "debug")])
    );
}

#[test]
fn build_env_vars_with_arg() {
    let t = task_with_env("run", "cmd", vec![("BIN", "{bin}")], vec!["bin"], vec![]);
    assert_eq!(
        t.build_env_vars(&[s("doer")], &no_overrides()).unwrap(),
        h(&[("BIN", "doer")])
    );
}

#[test]
fn build_env_vars_with_opt() {
    let t = task_with_env(
        "run",
        "cmd",
        vec![("MODE", "{mode}")],
        vec![],
        vec![("mode", "release")],
    );
    assert_eq!(
        t.build_env_vars(&[], &no_overrides()).unwrap(),
        h(&[("MODE", "release")])
    );
}

#[test]
fn build_env_vars_with_opt_override() {
    let t = task_with_env("run", "cmd", vec![("MODE", "{mode}")], vec![], vec![("mode", "debug")]);
    assert_eq!(
        t.build_env_vars(&[], &overrides(&[("mode", "release")])).unwrap(),
        h(&[("MODE", "release")])
    );
}

#[test]
fn build_env_vars_mixed() {
    let t = task_with_env(
        "run",
        "cmd",
        vec![("A", "{x}"), ("B", "{y}")],
        vec!["x"],
        vec![("y", "bar")],
    );
    assert_eq!(
        t.build_env_vars(&[s("foo")], &no_overrides()).unwrap(),
        h(&[("A", "foo"), ("B", "bar")])
    );
}

#[test]
fn build_env_vars_missing_arg() {
    let t = task_with_env("run", "cmd", vec![("BIN", "{bin}")], vec!["bin"], vec![]);
    let err = t.build_env_vars(&[], &no_overrides()).unwrap_err();
    assert!(format!("{:#}", err).contains("missing argument at position 0"));
}

// ===================================================================
// build_stdin / build_stdout / build_stderr
// ===================================================================

#[test]
fn stdio_default_when_not_configured() {
    let t = task("run", "cmd", vec![], vec![]);
    assert_eq!(t.build_stdin(&[], &no_overrides()).unwrap(), doer_spec::StdIo::Inherit);
    assert_eq!(t.build_stdout(&[], &no_overrides()).unwrap(), doer_spec::StdIo::Inherit);
    assert_eq!(t.build_stderr(&[], &no_overrides()).unwrap(), doer_spec::StdIo::Inherit);
}

#[test]
fn stdio_literal_values() {
    let t = task_with_stdio(
        "run",
        "cmd",
        Some("null"),
        Some("inherit"),
        Some("void"),
        vec![],
        vec![],
    );
    assert_eq!(t.build_stdin(&[], &no_overrides()).unwrap(), doer_spec::StdIo::Null);
    assert_eq!(t.build_stdout(&[], &no_overrides()).unwrap(), doer_spec::StdIo::Inherit);
    assert_eq!(t.build_stderr(&[], &no_overrides()).unwrap(), doer_spec::StdIo::Null);
}

#[test]
fn stdio_with_opt_variable() {
    let t = task_with_stdio("run", "cmd", Some("{mode}"), None, None, vec![], vec![("mode", "null")]);
    assert_eq!(t.build_stdin(&[], &no_overrides()).unwrap(), doer_spec::StdIo::Null);
}

#[test]
fn stdio_with_opt_override() {
    let t = task_with_stdio(
        "run",
        "cmd",
        Some("{mode}"),
        None,
        None,
        vec![],
        vec![("mode", "inherit")],
    );
    assert_eq!(
        t.build_stdin(&[], &overrides(&[("mode", "null")])).unwrap(),
        doer_spec::StdIo::Null
    );
}

#[test]
fn stdio_with_arg_variable() {
    let t = task_with_stdio("run", "cmd", Some("{mode}"), None, None, vec!["mode"], vec![]);
    assert_eq!(
        t.build_stdin(&[s("void")], &no_overrides()).unwrap(),
        doer_spec::StdIo::Null
    );
}

#[test]
fn stdio_invalid_value_after_resolution() {
    let t = task_with_stdio("run", "cmd", Some("invalid"), None, None, vec![], vec![]);
    let err = t.build_stdin(&[], &no_overrides()).unwrap_err();
    assert!(format!("{:#}", err).contains("invalid stdin value after resolution"));
}

// ===================================================================
// build_dep — dep args/opts are independent of parent
// ===================================================================

fn task_with_stdio(
    name: &str,
    command: &str,
    stdin: Option<&str>,
    stdout: Option<&str>,
    stderr: Option<&str>,
    args: Vec<&str>,
    opts: Vec<(&str, &str)>,
) -> Task {
    Task {
        name: name.to_string(),
        commands: vec![command.to_string()],
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: None,
        env_vars: vec![],
        opts: opts
            .iter()
            .map(|(k, v)| Opt {
                name: k.to_string(),
                value: v.to_string(),
            })
            .collect(),
        deps: vec![],
        user: None,
        stdin: stdin.map(|s| s.to_string()),
        stdout: stdout.map(|s| s.to_string()),
        stderr: stderr.map(|s| s.to_string()),
    }
}

fn dep_with_args(name: &str, args: Vec<&str>) -> Dep {
    Dep {
        name: name.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
        opts: vec![],
        background: false,
        stdin: None,
        stdout: None,
        stderr: None,
    }
}

fn dep_with_opts(name: &str, opts: Vec<(&str, &str)>) -> Dep {
    Dep {
        name: name.to_string(),
        args: vec![],
        opts: opts
            .iter()
            .map(|(k, v)| Opt {
                name: k.to_string(),
                value: v.to_string(),
            })
            .collect(),
        background: false,
        stdin: None,
        stdout: None,
        stderr: None,
    }
}

fn config(tasks: Vec<Task>) -> Config {
    Config { tasks, git_hooks: None }
}

// --- build_dep: literal dep args, no parent reference ---

#[test]
fn build_dep_literal_args_independent_of_parent() {
    let parent = task_with_deps(
        "parent",
        "echo {x}",
        vec!["x"],
        vec![],
        vec![dep_with_args("child", vec!["hello"])],
    );

    let r = parent.build_dep(&parent.deps[0], &[s("goodbye")], &no_overrides()).unwrap();

    assert_eq!(r.args, vec!["hello"]);
    assert!(r.opt_overrides.is_empty());
}

// --- build_dep: dep arg references parent variable ---

#[test]
fn build_dep_references_parent_arg() {
    let parent = task_with_deps(
        "parent",
        "echo {x}",
        vec!["x"],
        vec![],
        vec![dep_with_args("child", vec!["{x}"])],
    );

    let r = parent.build_dep(&parent.deps[0], &[s("goodbye")], &no_overrides()).unwrap();

    assert_eq!(r.args, vec!["goodbye"]);
}

#[test]
fn build_dep_references_parent_opt() {
    let parent = task_with_deps(
        "parent",
        "echo {x}",
        vec![],
        vec![("x", "hello")],
        vec![dep_with_args("child", vec!["{x}"])],
    );

    let r = parent.build_dep(&parent.deps[0], &[], &no_overrides()).unwrap();

    assert_eq!(r.args, vec!["hello"]);
}

#[test]
fn build_dep_literal_and_ref_mixed() {
    let parent = task_with_deps(
        "parent",
        "echo {x} {y}",
        vec!["x"],
        vec![("y", "bar")],
        vec![Dep {
            name: "child".into(),
            args: vec!["literal".into(), "{x}".into(), "{y}".into()],
            opts: vec![],
            background: false,
            stdin: None,
            stdout: None,
            stderr: None,
        }],
    );

    let r = parent.build_dep(&parent.deps[0], &[s("foo")], &no_overrides()).unwrap();

    assert_eq!(r.args, vec!["literal", "foo", "bar"]);
}

// --- build_dep: dep opts are independent ---

#[test]
fn build_dep_own_opts() {
    let parent = task_with_deps(
        "parent",
        "cmd",
        vec![],
        vec![("debug", "false")],
        vec![dep_with_opts("child", vec![("mode", "release")])],
    );

    let r = parent.build_dep(&parent.deps[0], &[], &no_overrides()).unwrap();

    assert!(r.args.is_empty());
    assert_eq!(r.opt_overrides.get("mode").unwrap(), "release");
}

#[test]
fn build_dep_opt_references_parent_var() {
    let parent = task_with_deps(
        "parent",
        "cmd",
        vec![],
        vec![("debug", "false")],
        vec![dep_with_opts("child", vec![("mode", "{debug}")])],
    );

    let r = parent.build_dep(&parent.deps[0], &[], &no_overrides()).unwrap();

    assert_eq!(r.opt_overrides.get("mode").unwrap(), "false");
}

#[test]
fn build_dep_opt_does_not_leak_parent_opts() {
    let parent = task_with_deps(
        "parent",
        "cmd",
        vec![],
        vec![("debug", "false"), ("other", "x")],
        vec![dep_with_opts("child", vec![("mode", "release")])],
    );

    let r = parent.build_dep(&parent.deps[0], &[], &no_overrides()).unwrap();

    assert_eq!(r.opt_overrides.len(), 1);
    assert_eq!(r.opt_overrides.get("mode").unwrap(), "release");
    assert!(!r.opt_overrides.contains_key("debug"));
    assert!(!r.opt_overrides.contains_key("other"));
}

// ===================================================================
// build_task_with_deps — each task gets its own independent args/opts
// ===================================================================

#[test]
fn each_task_has_its_own_args() {
    let parent_task = task_with_deps(
        "parent",
        "echo {x}",
        vec!["x"],
        vec![],
        vec![dep_with_args("child", vec!["hello"])],
    );
    let child_task = task("child", "echo {y}", vec!["y"], vec![]);
    let cfg = config(vec![parent_task, child_task]);

    let resolved = cfg.build_task_with_deps("parent", &[s("goodbye")], &no_overrides()).unwrap();

    assert_eq!(resolved.len(), 2);

    let child = &resolved[0];
    assert_eq!(child.task.name, "child");
    assert_eq!(child.args.as_slice(), &["hello"]);
    assert!(child.opt_overrides.is_empty());
    assert_eq!(
        child.task.build_commands(&child.args, &child.opt_overrides).unwrap(),
        vec!["echo hello"]
    );

    let parent = &resolved[1];
    assert_eq!(parent.task.name, "parent");
    assert_eq!(parent.args.as_slice(), &["goodbye"]);
    assert!(parent.opt_overrides.is_empty());
    assert_eq!(
        parent.task.build_commands(&parent.args, &parent.opt_overrides).unwrap(),
        vec!["echo goodbye"]
    );
}

#[test]
fn dep_references_parent_value_but_has_own_args() {
    let parent_task = task_with_deps(
        "parent",
        "echo parent-{x}",
        vec!["x"],
        vec![],
        vec![dep_with_args("child", vec!["{x}"])],
    );
    let child_task = task("child", "echo child-{y}", vec!["y"], vec![]);
    let cfg = config(vec![parent_task, child_task]);

    let resolved = cfg.build_task_with_deps("parent", &[s("world")], &no_overrides()).unwrap();

    assert_eq!(resolved.len(), 2);

    let child = &resolved[0];
    assert_eq!(
        child.task.build_commands(&child.args, &no_overrides()).unwrap(),
        vec!["echo child-world"]
    );

    let parent = &resolved[1];
    assert_eq!(
        parent.task.build_commands(&parent.args, &no_overrides()).unwrap(),
        vec!["echo parent-world"]
    );
}

#[test]
fn dep_opt_overrides_are_independent() {
    let parent_task = task_with_deps(
        "parent",
        "echo {a}",
        vec!["a"],
        vec![("b", "default")],
        vec![dep_with_opts("child", vec![("b", "overridden")])],
    );
    let child_task = Task {
        name: "child".into(),
        commands: vec!["echo {b}".into()],
        args: vec![],
        cwd: Some("target/{b}".into()),
        env_vars: vec![],
        opts: vec![Opt {
            name: "b".into(),
            value: "child_default".into(),
        }],
        deps: vec![],
        user: None,
        stdin: None,
        stdout: None,
        stderr: None,
    };
    let cfg = config(vec![parent_task, child_task]);

    let resolved = cfg.build_task_with_deps("parent", &[s("foo")], &no_overrides()).unwrap();

    assert_eq!(resolved.len(), 2);

    let child = &resolved[0];
    assert_eq!(
        child.task.build_commands(&child.args, &child.opt_overrides).unwrap(),
        vec!["echo overridden"]
    );
    assert_eq!(
        child.task.build_cwd(&child.args, &child.opt_overrides).unwrap(),
        Some("target/overridden".into())
    );

    let parent = &resolved[1];
    assert_eq!(
        parent.task.build_commands(&parent.args, &parent.opt_overrides).unwrap(),
        vec!["echo foo"]
    );
}

#[test]
fn multiple_deps_each_with_different_args() {
    let parent_task = task_with_deps(
        "parent",
        "echo {x}",
        vec!["x"],
        vec![],
        vec![
            dep_with_args("child_a", vec!["aaa"]),
            dep_with_args("child_b", vec!["{x}"]),
        ],
    );
    let child_a = task("child_a", "echo child_a-{y}", vec!["y"], vec![]);
    let child_b = task("child_b", "echo child_b-{y}", vec!["y"], vec![]);
    let cfg = config(vec![parent_task, child_a, child_b]);

    let resolved = cfg.build_task_with_deps("parent", &[s("world")], &no_overrides()).unwrap();

    assert_eq!(resolved.len(), 3);

    let child_a = &resolved[0];
    assert_eq!(
        child_a.task.build_commands(&child_a.args, &no_overrides()).unwrap(),
        vec!["echo child_a-aaa"]
    );

    let child_b = &resolved[1];
    assert_eq!(
        child_b.task.build_commands(&child_b.args, &no_overrides()).unwrap(),
        vec!["echo child_b-world"]
    );

    let parent = &resolved[2];
    assert_eq!(
        parent.task.build_commands(&parent.args, &no_overrides()).unwrap(),
        vec!["echo world"]
    );
}

#[test]
fn transitive_dep_gets_own_args() {
    let grandparent = task_with_deps(
        "grandparent",
        "echo {x}",
        vec!["x"],
        vec![],
        vec![dep_with_args("parent", vec!["literal"])],
    );
    let parent = task_with_deps(
        "parent",
        "echo {y}",
        vec!["y"],
        vec![],
        vec![dep_with_args("child", vec!["from-parent"])],
    );
    let child = task("child", "echo {z}", vec!["z"], vec![]);
    let cfg = config(vec![grandparent, parent, child]);

    let resolved = cfg.build_task_with_deps("grandparent", &[s("top")], &no_overrides()).unwrap();

    assert_eq!(resolved.len(), 3);

    let child_t = &resolved[0];
    assert_eq!(child_t.task.name, "child");
    assert_eq!(child_t.args.as_slice(), &["from-parent"]);

    let parent_t = &resolved[1];
    assert_eq!(parent_t.task.name, "parent");
    assert_eq!(parent_t.args.as_slice(), &["literal"]);

    let gp_t = &resolved[2];
    assert_eq!(gp_t.task.name, "grandparent");
    assert_eq!(gp_t.args.as_slice(), &["top"]);
}

#[test]
fn dep_args_are_fully_independent_vec() {
    let parent_task = task_with_deps(
        "parent",
        "echo {a} {b}",
        vec!["a", "b"],
        vec![],
        vec![dep_with_args("child", vec!["one", "two"])],
    );
    let child_task = task("child", "echo {p} {q}", vec!["p", "q"], vec![]);
    let cfg = config(vec![parent_task, child_task]);

    let resolved = cfg.build_task_with_deps("parent", &[s("x"), s("y")], &no_overrides()).unwrap();

    let child = &resolved[0];
    assert_eq!(child.args.as_slice(), &["one", "two"]);
    assert_eq!(
        child.task.build_commands(&child.args, &no_overrides()).unwrap(),
        vec!["echo one two"]
    );

    let parent = &resolved[1];
    assert_eq!(parent.args.as_slice(), &["x", "y"]);
}

// ===================================================================
// build_task_with_deps — topological order
// ===================================================================

fn simple_task(name: &str) -> Task {
    task(name, "cmd", vec![], vec![])
}

fn simple_task_with_deps(name: &str, deps: Vec<Dep>) -> Task {
    task_with_deps(name, "cmd", vec![], vec![], deps)
}

fn task_names(resolved: &[TaskCall<'_>]) -> Vec<String> {
    resolved.iter().map(|c| c.task.name.clone()).collect()
}

#[test]
fn order_simple_chain() {
    // a -> b -> c
    let c = simple_task("c");
    let b = simple_task_with_deps("b", vec![dep_with_args("c", vec![])]);
    let a = simple_task_with_deps("a", vec![dep_with_args("b", vec![])]);
    let cfg = config(vec![a, b, c]);

    let resolved = cfg.build_task_with_deps("a", &[], &no_overrides()).unwrap();

    assert_eq!(task_names(&resolved), vec!["c", "b", "a"]);
}

#[test]
fn order_tree() {
    // a -> b, c  ;  b -> d
    let d = simple_task("d");
    let c = simple_task("c");
    let b = simple_task_with_deps("b", vec![dep_with_args("d", vec![])]);
    let a = simple_task_with_deps("a", vec![dep_with_args("b", vec![]), dep_with_args("c", vec![])]);
    let cfg = config(vec![a, b, c, d]);

    let resolved = cfg.build_task_with_deps("a", &[], &no_overrides()).unwrap();

    // d before b (b->d), b before a (a->b), c before a (a->c)
    // b subtree complete before moving to c
    assert_eq!(task_names(&resolved), vec!["d", "b", "c", "a"]);
}

#[test]
fn order_diamond_dep_deduplicated() {
    // a -> b1, b2  ;  b1 -> c  ;  b2 -> c
    let c = simple_task("c");
    let b1 = simple_task_with_deps("b1", vec![dep_with_args("c", vec![])]);
    let b2 = simple_task_with_deps("b2", vec![dep_with_args("c", vec![])]);
    let a = simple_task_with_deps("a", vec![dep_with_args("b1", vec![]), dep_with_args("b2", vec![])]);
    let cfg = config(vec![a, b1, b2, c]);

    let resolved = cfg.build_task_with_deps("a", &[], &no_overrides()).unwrap();

    // c appears only once, and before its dependents (b1, b2)
    assert_eq!(task_names(&resolved), vec!["c", "b1", "b2", "a"]);
}

#[test]
fn order_deeper_nesting() {
    // a -> b ;  b -> c ;  c -> d, e
    let d = simple_task("d");
    let e = simple_task("e");
    let c = simple_task_with_deps("c", vec![dep_with_args("d", vec![]), dep_with_args("e", vec![])]);
    let b = simple_task_with_deps("b", vec![dep_with_args("c", vec![])]);
    let a = simple_task_with_deps("a", vec![dep_with_args("b", vec![])]);
    let cfg = config(vec![a, b, c, d, e]);

    let resolved = cfg.build_task_with_deps("a", &[], &no_overrides()).unwrap();

    // dep before dependent at every level
    assert_eq!(task_names(&resolved), vec!["d", "e", "c", "b", "a"]);
}

#[test]
fn order_shared_dep_at_different_levels() {
    // a -> b, c ;  b -> c
    // c is both a dep of a and a dep of b
    let c = simple_task("c");
    let b = simple_task_with_deps("b", vec![dep_with_args("c", vec![])]);
    let a = simple_task_with_deps("a", vec![dep_with_args("b", vec![]), dep_with_args("c", vec![])]);
    let cfg = config(vec![a, b, c]);

    let resolved = cfg.build_task_with_deps("a", &[], &no_overrides()).unwrap();

    // c is resolved once (as dep of b), then b, then a
    // c as direct dep of a is skipped (already resolved)
    assert_eq!(task_names(&resolved), vec!["c", "b", "a"]);
}

// ===================================================================
// background flag propagation
// ===================================================================

#[test]
fn build_all_propagates_background_flag() {
    let dep = Dep {
        name: "child".into(),
        args: vec![],
        opts: vec![],
        background: true,
        stdin: None,
        stdout: None,
        stderr: None,
    };
    let parent_task = task_with_deps("parent", "echo parent", vec![], vec![], vec![dep]);
    let child_task = task("child", "echo child", vec![], vec![]);
    let cfg = config(vec![parent_task, child_task]);

    let runnables = cfg.build_all("parent", &[], &no_overrides()).unwrap();

    assert_eq!(runnables.len(), 2);
    assert!(runnables[0].background);
    assert_eq!(runnables[0].name, "child");
    assert!(!runnables[1].background);
    assert_eq!(runnables[1].name, "parent");
}

// ===================================================================
// dep stdio override
// ===================================================================

#[test]
fn dep_stdio_literal_override() {
    let child = Task {
        name: "child".into(),
        commands: vec!["echo child".into()],
        args: vec![],
        cwd: None,
        env_vars: vec![],
        opts: vec![],
        deps: vec![],
        user: None,
        stdin: Some("inherit".into()),
        stdout: Some("inherit".into()),
        stderr: Some("inherit".into()),
    };
    let parent = Task {
        name: "parent".into(),
        commands: vec!["echo parent".into()],
        args: vec![],
        cwd: None,
        env_vars: vec![],
        opts: vec![],
        deps: vec![Dep {
            name: "child".into(),
            args: vec![],
            opts: vec![],
            background: false,
            stdin: Some("void".into()),
            stdout: Some("null".into()),
            stderr: None,
        }],
        user: None,
        stdin: None,
        stdout: None,
        stderr: None,
    };
    let cfg = config(vec![parent, child]);
    let runnables = cfg.build_all("parent", &[], &no_overrides()).unwrap();

    assert_eq!(runnables.len(), 2);

    // dep overrides child's stdin and stdout
    assert_eq!(runnables[0].stdin, doer_spec::StdIo::Null); // "void" -> Null
    assert_eq!(runnables[0].stdout, doer_spec::StdIo::Null); // "null" -> Null
    assert_eq!(runnables[0].stderr, doer_spec::StdIo::Inherit); // not overridden, uses child's own

    // parent unaffected
    assert_eq!(runnables[1].stdin, doer_spec::StdIo::Inherit);
    assert_eq!(runnables[1].stdout, doer_spec::StdIo::Inherit);
    assert_eq!(runnables[1].stderr, doer_spec::StdIo::Inherit);
}

#[test]
fn dep_stdio_override_resolves_parent_variable() {
    let child = Task {
        name: "child".into(),
        commands: vec!["echo child".into()],
        args: vec![],
        cwd: None,
        env_vars: vec![],
        opts: vec![],
        deps: vec![],
        user: None,
        stdin: None,
        stdout: None,
        stderr: None,
    };
    let parent = Task {
        name: "parent".into(),
        commands: vec!["echo parent".into()],
        args: vec![],
        cwd: None,
        env_vars: vec![],
        opts: vec![Opt {
            name: "mode".into(),
            value: "null".into(),
        }],
        deps: vec![Dep {
            name: "child".into(),
            args: vec![],
            opts: vec![],
            background: false,
            stdin: None,
            stdout: Some("{mode}".into()),
            stderr: None,
        }],
        user: None,
        stdin: None,
        stdout: None,
        stderr: None,
    };
    let cfg = config(vec![parent, child]);
    let runnables = cfg.build_all("parent", &[], &no_overrides()).unwrap();

    assert_eq!(runnables[0].stdout, doer_spec::StdIo::Null);
}

// ===================================================================
// no-command tasks (grouping task)
// ===================================================================

fn no_command_task(name: &str, deps: Vec<Dep>) -> Task {
    Task {
        name: name.to_string(),
        commands: vec![],
        args: vec![],
        cwd: None,
        env_vars: vec![],
        opts: vec![],
        deps,
        user: None,
        stdin: None,
        stdout: None,
        stderr: None,
    }
}

#[test]
fn no_command_task_with_dep_builds_dep_only() {
    let child = task("child", "echo hello", vec![], vec![]);
    let parent = no_command_task("parent", vec![dep_with_args("child", vec![])]);
    let cfg = config(vec![parent, child]);

    let resolved = cfg.build_task_with_deps("parent", &[], &no_overrides()).unwrap();
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0].task.name, "child");
    assert_eq!(resolved[1].task.name, "parent");

    let runnables = cfg.build_all("parent", &[], &no_overrides()).unwrap();
    assert_eq!(runnables.len(), 2);
    assert!(!runnables[0].commands.is_empty());
    assert_eq!(runnables[0].name, "child");
    assert!(runnables[1].commands.is_empty());
    assert_eq!(runnables[1].name, "parent");
}

#[test]
fn no_command_task_inherits_dep_order() {
    let d = task("d", "echo d", vec![], vec![]);
    let c = task("c", "echo c", vec![], vec![]);
    let b = simple_task_with_deps("b", vec![dep_with_args("c", vec![]), dep_with_args("d", vec![])]);
    let a = simple_task_with_deps("a", vec![dep_with_args("b", vec![])]);
    let group = no_command_task("group", vec![dep_with_args("a", vec![]), dep_with_args("b", vec![])]);
    let cfg = config(vec![group, a, b, c, d]);

    let resolved = cfg.build_task_with_deps("group", &[], &no_overrides()).unwrap();

    assert_eq!(task_names(&resolved), vec!["c", "d", "b", "a", "group"]);
}
