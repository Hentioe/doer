use doer_parser::config::*;
use kdl::{KdlDocument, KdlNode};

fn first_node(doc: &KdlDocument) -> &KdlNode {
    doc.nodes().first().unwrap()
}

fn parse_doc(kdl: &str) -> KdlDocument {
    kdl.parse().unwrap()
}

fn task_node(kdl: &str) -> KdlNode {
    let doc = parse_doc(kdl);
    doc.nodes().first().unwrap().clone()
}

// ===================================================================
// parse_commands
// ===================================================================

mod command_simple {
    use super::*;

    #[test]
    fn single_entry() {
        let doc = parse_doc(r#"build "cargo build""#);
        let node = first_node(&doc);
        assert_eq!(parse_commands(&node, "test").unwrap(), vec!["cargo build"]);
    }

    #[test]
    fn too_many_entries() {
        let doc = parse_doc(r#"build "cargo build" "extra""#);
        let node = first_node(&doc);
        let err = parse_commands(&node, "test").unwrap_err();
        assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
    }

    #[test]
    fn non_string_entry() {
        let doc = parse_doc("build 42");
        let node = first_node(&doc);
        let err = parse_commands(&node, "test").unwrap_err();
        assert!(format!("{:#}", err).contains("command is not a string"));
    }
}

mod command_complex {
    use super::*;

    #[test]
    fn single_dash() {
        let doc = parse_doc(r#"run { - "cargo run" }"#);
        let node = first_node(&doc);
        assert_eq!(parse_commands(&node, "test").unwrap(), vec!["cargo run"]);
    }

    #[test]
    fn no_dash() {
        let doc = parse_doc("run { }");
        let node = first_node(&doc);
        let result = parse_commands(&node, "test").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn multiple_dash() {
        let doc = parse_doc(
            r#"run {
                - "cmd1"
                - "cmd2"
            }"#,
        );
        let node = first_node(&doc);
        assert_eq!(parse_commands(&node, "test").unwrap(), vec!["cmd1", "cmd2"]);
    }

    #[test]
    fn dash_no_entry() {
        let doc = parse_doc("run { - }");
        let node = first_node(&doc);
        let err = parse_commands(&node, "test").unwrap_err();
        assert!(format!("{:#}", err).contains("expected 1 entries, got 0"));
    }
}

#[test]
fn command_no_entries_and_no_children() {
    let doc = parse_doc("empty");
    let node = first_node(&doc);
    let result = parse_commands(&node, "test").unwrap();
    assert!(result.is_empty());
}

// ===================================================================
// parse_arg
// ===================================================================

#[test]
fn arg_valid() {
    let doc = parse_doc(r#"arg "path""#);
    let node = first_node(&doc);
    assert_eq!(parse_arg(node, "test").unwrap(), "path");
}

#[test]
fn arg_no_entry() {
    let doc = parse_doc("arg");
    let node = first_node(&doc);
    let err = parse_arg(node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 0"));
}

#[test]
fn arg_too_many_entries() {
    let doc = parse_doc(r#"arg "a" "b""#);
    let node = first_node(&doc);
    let err = parse_arg(node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
}

// ===================================================================
// parse_opt
// ===================================================================

#[test]
fn opt_valid() {
    let doc = parse_doc(r#"opt mode="debug""#);
    let node = first_node(&doc);
    let opt = parse_opt(node, "test").unwrap();
    assert_eq!(opt.name, "mode");
    assert!(matches!(opt.value, OptValue::String(v) if v == "debug"));
}

#[test]
fn opt_no_entry() {
    let doc = parse_doc("opt");
    let node = first_node(&doc);
    let err = parse_opt(node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 0"));
}

#[test]
fn opt_too_many_entries() {
    let doc = parse_doc(r#"opt mode="debug" extra="y""#);
    let node = first_node(&doc);
    let err = parse_opt(node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
}

#[test]
fn opt_no_key() {
    let doc = parse_doc(r#"opt "debug""#);
    let node = first_node(&doc);
    let err = parse_opt(node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("opt has no key"));
}

#[test]
fn opt_integer_value() {
    let doc = parse_doc(r#"opt count=5"#);
    let node = first_node(&doc);
    let opt = parse_opt(node, "test").unwrap();
    assert_eq!(opt.name, "count");
    assert!(matches!(opt.value, OptValue::String(v) if v == "5"));
}

#[test]
fn opt_float_value() {
    let doc = parse_doc(r#"opt ratio=3.14"#);
    let node = first_node(&doc);
    let opt = parse_opt(node, "test").unwrap();
    assert_eq!(opt.name, "ratio");
    assert!(matches!(opt.value, OptValue::String(v) if v == "3.14"));
}

#[test]
fn opt_negative_integer_value() {
    let doc = parse_doc(r#"opt offset=-1"#);
    let node = first_node(&doc);
    let opt = parse_opt(node, "test").unwrap();
    assert_eq!(opt.name, "offset");
    assert!(matches!(opt.value, OptValue::String(v) if v == "-1"));
}

#[test]
fn opt_bool_true_value() {
    let doc = parse_doc(r#"opt flag=#true"#);
    let node = first_node(&doc);
    let opt = parse_opt(node, "test").unwrap();
    assert_eq!(opt.name, "flag");
    assert!(matches!(opt.value, OptValue::Bool(true)));
}

#[test]
fn opt_bool_false_value() {
    let doc = parse_doc(r#"opt flag=#false"#);
    let node = first_node(&doc);
    let opt = parse_opt(node, "test").unwrap();
    assert_eq!(opt.name, "flag");
    assert!(matches!(opt.value, OptValue::Bool(false)));
}

#[test]
fn opt_non_bool_not_marked_boolean() {
    let doc = parse_doc(r#"opt mode="debug""#);
    let node = first_node(&doc);
    let opt = parse_opt(node, "test").unwrap();
    assert_eq!(opt.name, "mode");
    assert!(matches!(opt.value, OptValue::String(v) if v == "debug"));
}

#[test]
fn opt_integer_not_marked_boolean() {
    let doc = parse_doc(r#"opt count=5"#);
    let node = first_node(&doc);
    let opt = parse_opt(node, "test").unwrap();
    assert!(matches!(opt.value, OptValue::String(_)));
}

// ===================================================================
// parse_dep
// ===================================================================

#[test]
fn dep_simple() {
    let doc = parse_doc(r#"dep "test""#);
    let node = first_node(&doc);
    let dep = parse_dep(node).unwrap();
    assert_eq!(dep.name, "test");
    assert!(dep.args.is_empty());
}

#[test]
fn dep_with_args() {
    let doc = parse_doc(r#"dep "add-cap" { arg "$bin" }"#);
    let node = first_node(&doc);
    let dep = parse_dep(node).unwrap();
    assert_eq!(dep.name, "add-cap");
    assert_eq!(dep.args, vec!["$bin"]);
}

#[test]
fn dep_no_entry() {
    let doc = parse_doc("dep");
    let node = first_node(&doc);
    let err = parse_dep(node).unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 0"));
}

#[test]
fn dep_too_many_entries() {
    let doc = parse_doc(r#"dep "a" "b""#);
    let node = first_node(&doc);
    let err = parse_dep(node).unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
}

// ===================================================================
// parse_dep_arg
// ===================================================================

#[test]
fn dep_arg_valid() {
    let doc = parse_doc(r#"arg "$bin""#);
    let node = first_node(&doc);
    assert_eq!(parse_dep_arg(node, "my-dep").unwrap(), "$bin");
}

#[test]
fn dep_arg_too_many() {
    let doc = parse_doc(r#"arg "$bin" "extra""#);
    let node = first_node(&doc);
    let err = parse_dep_arg(node, "my-dep").unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
}

// ===================================================================
// parse_optional_string
// ===================================================================

#[test]
fn optional_present() {
    let node = task_node(r#"task { user "root" }"#);
    assert_eq!(
        parse_optional_string(&node, "test", "user").unwrap(),
        Some("root".into())
    );
}

#[test]
fn optional_absent() {
    let node = task_node(r#"task { arg "x" }"#);
    assert_eq!(parse_optional_string(&node, "test", "user").unwrap(), None);
}

#[test]
fn optional_no_children_block() {
    let node = task_node(r#"task "cmd""#);
    assert_eq!(parse_optional_string(&node, "test", "user").unwrap(), None);
}

#[test]
fn optional_duplicate() {
    let node = task_node(r#"task { user "root"; user "admin" }"#);
    let err = parse_optional_string(&node, "test", "user").unwrap_err();
    assert!(format!("{:#}", err).contains("expected at most 1 user node, got 2"));
}

#[test]
fn optional_too_many_entries() {
    let node = task_node(r#"task { user "root" "admin" }"#);
    let err = parse_optional_string(&node, "test", "user").unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
}

// ===================================================================
// parse_env_var
// ===================================================================

#[test]
fn env_var_valid() {
    let doc = parse_doc(r#"KEY "value""#);
    let node = first_node(&doc);
    let var = parse_env_var(node, "test").unwrap();
    assert_eq!(var.name, "KEY");
    assert_eq!(var.value, "value");
}

#[test]
fn env_var_too_many_entries() {
    let doc = parse_doc(r#"KEY "v1" "v2""#);
    let node = first_node(&doc);
    let err = parse_env_var(node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
}

#[test]
fn env_var_integer_value() {
    let doc = parse_doc("PORT 8080");
    let node = first_node(&doc);
    let var = parse_env_var(node, "test").unwrap();
    assert_eq!(var.name, "PORT");
    assert_eq!(var.value, "8080");
}

#[test]
fn env_var_float_value() {
    let doc = parse_doc("RATIO 3.14");
    let node = first_node(&doc);
    let var = parse_env_var(node, "test").unwrap();
    assert_eq!(var.name, "RATIO");
    assert_eq!(var.value, "3.14");
}

#[test]
fn env_var_negative_integer_value() {
    let doc = parse_doc("OFFSET -1");
    let node = first_node(&doc);
    let var = parse_env_var(node, "test").unwrap();
    assert_eq!(var.name, "OFFSET");
    assert_eq!(var.value, "-1");
}

#[test]
fn env_var_bool_value_is_rejected() {
    let doc = parse_doc("FLAG #true");
    let node = first_node(&doc);
    let err = parse_env_var(node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("not a string or number"));
}

// ===================================================================
// parse_env_vars (container)
// ===================================================================

#[test]
fn env_vars_container() {
    let node = task_node(r#"task { env { KEY "val" } }"#);
    let vars = parse_env_vars(&node, "test").unwrap();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "KEY");
    assert_eq!(vars[0].value, "val");
}

#[test]
fn env_vars_multiple() {
    let node = task_node(r#"task { env { A "1"; B "2" } }"#);
    let vars = parse_env_vars(&node, "test").unwrap();
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].name, "A");
    assert_eq!(vars[1].name, "B");
}

#[test]
fn env_vars_duplicate_container() {
    let node = task_node(r#"task { env { A "1" }; env { B "2" } }"#);
    let err = parse_env_vars(&node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("expected at most 1 env node, got 2"));
}

// ===================================================================
// parse_args / parse_opts / parse_deps (multi-value collections)
// ===================================================================

#[test]
fn args_multiple() {
    let node = task_node(r#"task { arg "a"; arg "b" }"#);
    let args = parse_args(&node, "test").unwrap();
    assert_eq!(args, vec!["a", "b"]);
}

#[test]
fn args_empty() {
    let node = task_node(r#"task { opt k="v" }"#);
    let args = parse_args(&node, "test").unwrap();
    assert!(args.is_empty());
}

#[test]
fn opts_multiple() {
    let node = task_node(r#"task { opt a="1"; opt b="2" }"#);
    let opts = parse_opts(&node, "test").unwrap();
    assert_eq!(opts.len(), 2);
    assert_eq!(opts[0].name, "a");
    assert_eq!(opts[1].name, "b");
}

#[test]
fn deps_multiple() {
    let node = task_node(r#"task { dep "a"; dep "b" { arg "$x" } }"#);
    let deps = parse_deps(&node, "test").unwrap();
    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0].name, "a");
    assert!(deps[0].args.is_empty());
    assert_eq!(deps[1].name, "b");
    assert_eq!(deps[1].args, vec!["$x"]);
}

#[test]
fn deps_invalid_arg_in_dep() {
    let node = task_node(r#"task { dep "a" { arg "x" "y" } }"#);
    let err = parse_deps(&node, "test").unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
}

// ===================================================================
// parse_dep_background
// ===================================================================

#[test]
fn dep_background_default() {
    let doc = parse_doc(r#"dep "test""#);
    let node = first_node(&doc);
    let dep = parse_dep(node).unwrap();
    assert!(!dep.background);
}

#[test]
fn dep_background_without_value() {
    let doc = parse_doc(r#"dep "test" { background }"#);
    let node = first_node(&doc);
    let dep = parse_dep(node).unwrap();
    assert!(dep.background);
}

#[test]
fn dep_background_false() {
    let doc = parse_doc(r#"dep "test" { background #false }"#);
    let node = first_node(&doc);
    let dep = parse_dep(node).unwrap();
    assert!(!dep.background);
}

// ===================================================================
// from_kdl_str — no-command task validation
// ===================================================================

#[test]
fn no_command_with_deps_is_valid() {
    let cfg = Config::from_kdl_str(
        r#"tasks {
            prepush {
                dep precommit
                dep test
            }
            precommit { - "echo precommit" }
            test { - "echo test" }
        }"#,
    )
    .unwrap();
    assert_eq!(cfg.tasks.len(), 3);
    let prepush = &cfg.tasks[0];
    assert!(prepush.commands.is_empty());
    assert_eq!(prepush.deps.len(), 2);
    assert_eq!(prepush.deps[0].name, "precommit");
    assert_eq!(prepush.deps[1].name, "test");
}

#[test]
fn no_command_no_deps_is_error() {
    let err = Config::from_kdl_str(
        r#"tasks {
            empty { }
        }"#,
    )
    .unwrap_err();
    assert!(format!("{:#}", err).contains("has no command and no dependencies"));
}

#[test]
fn no_command_single_entry_no_deps_is_error() {
    let err = Config::from_kdl_str(
        r#"tasks {
            empty
        }"#,
    )
    .unwrap_err();
    assert!(format!("{:#}", err).contains("has no command and no dependencies"));
}

// ===================================================================
// parse_stdio
// ===================================================================
mod stdio_parsing {
    use super::*;

    #[test]
    fn all_default_when_omitted() {
        let doc = parse_doc(r#"build "cmd""#);
        let node = first_node(&doc);
        let (stdin, stdout, stderr) = parse_stdio(&node, "test").unwrap();
        assert!(stdin.is_none());
        assert!(stdout.is_none());
        assert!(stderr.is_none());
    }

    #[test]
    fn all_three_values() {
        let doc = parse_doc(
            r#"build "cmd" {
                stdin "null"
                stdout "inherit"
                stderr "void"
            }"#,
        );
        let node = first_node(&doc);
        let (stdin, stdout, stderr) = parse_stdio(&node, "test").unwrap();
        assert_eq!(stdin.unwrap(), "null");
        assert_eq!(stdout.unwrap(), "inherit");
        assert_eq!(stderr.unwrap(), "void");
    }

    #[test]
    fn partial_values() {
        let doc = parse_doc(
            r#"build "cmd" {
                stdout "default"
            }"#,
        );
        let node = first_node(&doc);
        let (stdin, stdout, stderr) = parse_stdio(&node, "test").unwrap();
        assert!(stdin.is_none());
        assert_eq!(stdout.unwrap(), "default");
        assert!(stderr.is_none());
    }

    #[test]
    fn duplicate_is_error() {
        let doc = parse_doc(
            r#"build "cmd" {
                stdout "inherit"
                stdout "null"
            }"#,
        );
        let node = first_node(&doc);
        let err = parse_stdio(&node, "test").unwrap_err();
        assert!(format!("{:#}", err).contains("expected at most 1"));
    }

    #[test]
    fn stores_raw_template() {
        let doc = parse_doc(
            r#"build "cmd" {
                stdout "{mode}"
            }"#,
        );
        let node = first_node(&doc);
        let (_stdin, stdout, _stderr) = parse_stdio(&node, "test").unwrap();
        assert_eq!(stdout.unwrap(), "{mode}");
    }
}

// ===================================================================
// parse_dep — stdio fields
// ===================================================================
mod dep_stdio_parsing {
    use super::*;

    fn parse_single_dep(kdl: &str) -> Dep {
        let doc = parse_doc(&format!(r#"task "cmd" {{ dep {} }}"#, kdl));
        let node = first_node(&doc);
        parse_deps(&node, "test").unwrap().into_iter().next().unwrap()
    }

    #[test]
    fn no_stdio_by_default() {
        let dep = parse_single_dep(r#""child""#);
        assert!(dep.stdin.is_none());
        assert!(dep.stdout.is_none());
        assert!(dep.stderr.is_none());
    }

    #[test]
    fn with_stdout_literal() {
        let dep = parse_single_dep(
            r#""child" {
                stdout "null"
            }"#,
        );
        assert_eq!(dep.stdout.unwrap(), "null");
    }

    #[test]
    fn with_stdin_and_stderr() {
        let dep = parse_single_dep(
            r#""child" {
                stdin "inherit"
                stderr "void"
            }"#,
        );
        assert_eq!(dep.stdin.unwrap(), "inherit");
        assert_eq!(dep.stderr.unwrap(), "void");
    }

    #[test]
    fn with_stdin_template() {
        let dep = parse_single_dep(
            r#""child" {
                stdin "{mode}"
            }"#,
        );
        assert_eq!(dep.stdin.unwrap(), "{mode}");
    }
}

// ===================================================================
// git-hooks parsing
// ===================================================================

#[test]
fn git_hooks_omitted_defaults_to_none() {
    let cfg = Config::from_kdl_str(
        r#"tasks {
            test { - "echo hi" }
        }"#,
    )
    .unwrap();
    assert!(cfg.git_hooks.is_none());
}

#[test]
fn git_hooks_true() {
    let cfg = Config::from_kdl_str(
        r#"git-hooks #true

        tasks {
            test { - "echo hi" }
        }"#,
    )
    .unwrap();
    assert_eq!(cfg.git_hooks, Some(true));
}

#[test]
fn git_hooks_false() {
    let cfg = Config::from_kdl_str(
        r#"git-hooks #false

        tasks {
            test { - "echo hi" }
        }"#,
    )
    .unwrap();
    assert_eq!(cfg.git_hooks, Some(false));
}

#[test]
fn git_hooks_non_bool_is_error() {
    let err = Config::from_kdl_str(
        r#"git-hooks "string"

        tasks {
            test { - "echo hi" }
        }"#,
    )
    .unwrap_err();
    assert!(format!("{:#}", err).contains("not a boolean"), "error: {err}");
}

#[test]
fn git_hooks_too_many_entries_is_error() {
    let err = Config::from_kdl_str(
        r#"git-hooks #true #false

        tasks {
            test { - "echo hi" }
        }"#,
    )
    .unwrap_err();
    assert!(format!("{:#}", err).contains("expected 1 entry"));
}
