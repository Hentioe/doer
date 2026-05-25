pub mod print;

#[doc(hidden)]
pub use colored; // 供 print 模块的宏使用

use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};
use typed_builder::TypedBuilder;

pub const NICE_MIN: i32 = -20;
pub const NICE_MAX: i32 = 19;
pub const VALID_STDIO_VALUES: &[&str] = &["default", "inherit", "null", "void"];

#[derive(Debug, TypedBuilder)]
pub struct Runnable {
    // 任务名，必要参数
    pub name: String,
    // 命令列表，必要参数
    pub commands: Vec<String>,
    #[builder(default)]
    pub cwd: Option<String>,
    #[builder(default)]
    pub env_vars: HashSet<EnvVar>,
    #[builder(default)]
    pub user: Option<String>,
    #[builder(default = SpecIo::Inherit)]
    pub stdin: SpecIo,
    #[builder(default = SpecIo::Inherit)]
    pub stdout: SpecIo,
    #[builder(default = SpecIo::Inherit)]
    pub stderr: SpecIo,
    #[builder(default)]
    pub nice: Option<i32>,
    #[builder(default = false)]
    pub background: bool,
}

#[derive(Debug)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpecIo {
    #[default]
    Inherit,
    Null,
}

impl TryFrom<&str> for SpecIo {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "inherit" | "default" => Ok(SpecIo::Inherit),
            "null" | "void" => Ok(SpecIo::Null),
            _ => Err(format!("invalid stdio value: {}", value)),
        }
    }
}

impl From<SpecIo> for std::process::Stdio {
    fn from(value: SpecIo) -> Self {
        match value {
            SpecIo::Inherit => std::process::Stdio::inherit(),
            SpecIo::Null => std::process::Stdio::null(),
        }
    }
}

impl PartialEq for EnvVar {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for EnvVar {}

impl Hash for EnvVar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
