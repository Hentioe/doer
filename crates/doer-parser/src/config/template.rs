use crate::prelude::*;
use doer_spec::EnvVar;
use std::collections::HashMap;

fn apply_template<F>(template: &str, label: &str, resolve: F) -> Result<String>
where
    F: Fn(&str) -> Result<String>,
{
    let mut result = String::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut inner = String::new();
            let mut closed = false;
            for ch in &mut chars {
                if ch == '}' {
                    closed = true;
                    break;
                }
                inner.push(ch);
            }
            ensure!(closed, "unclosed '{{' in {label}: {template}");
            result.push_str(&resolve(&inner)?);
        } else if ch == '}' {
            bail!("unexpected '}}' in {label}: {template}");
        } else {
            result.push(ch);
        }
    }
    Ok(result)
}

fn resolve_template_str(template: &str, ctx: &HashMap<String, String>) -> Result<String> {
    apply_template(template, "template", |name| {
        ensure!(!name.is_empty(), "empty placeholder '{{}}' in template");
        ctx.get(name).with_context(|| format!("undefined variable '{{{name}}}' in template")).cloned()
    })
}

fn resolve_args_str(template: &str, args: &[String], label: &str) -> Result<String> {
    apply_template(template, label, |num_str| {
        let index: usize =
            num_str.parse().with_context(|| format!("invalid positional placeholder '{{{num_str}}}' in {label}"))?;
        args.get(index)
            .with_context(|| {
                format!(
                    "missing argument at position {index} ({num_args} provided)",
                    num_args = args.len()
                )
            })
            .cloned()
    })
}

/// 可通过 substitution_context 和参数值列表分两阶段渲染的类型。
pub trait Templatable: Sized {
    /// 将 `{arg}` 替换为位置占位符，`{opt}` 替换为默认值。
    fn resolve_template(&self, ctx: &HashMap<String, String>) -> Result<Self>;

    /// 替换位置占位符 `{0}`, `{1}`... 为实际参数值。
    fn resolve_args(&self, args: &[String], label: &str) -> Result<Self>;

    /// 先应用 substitution_context，再替换位置占位符，一步到位。
    fn resolve_both(&self, ctx: &HashMap<String, String>, args: &[String]) -> Result<Self> {
        let tpl = self.resolve_template(ctx)?;
        tpl.resolve_args(args, "template")
    }
}

impl Templatable for String {
    fn resolve_template(&self, ctx: &HashMap<String, String>) -> Result<Self> {
        resolve_template_str(self, ctx)
    }

    fn resolve_args(&self, args: &[String], label: &str) -> Result<Self> {
        resolve_args_str(self, args, label)
    }
}

impl Templatable for EnvVar {
    fn resolve_template(&self, ctx: &HashMap<String, String>) -> Result<Self> {
        Ok(EnvVar {
            name: self.name.clone(),
            value: self.value.resolve_template(ctx)?,
        })
    }

    fn resolve_args(&self, args: &[String], label: &str) -> Result<Self> {
        Ok(EnvVar {
            name: self.name.clone(),
            value: self.value.resolve_args(args, label)?,
        })
    }
}
