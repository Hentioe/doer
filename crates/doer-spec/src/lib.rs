pub mod print;

#[doc(hidden)]
pub use colored; // 供 print 模块的宏使用

use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};
use typed_builder::TypedBuilder;

const VALID_STDIO_VALUES: &[&str] = &["default", "inherit", "null", "void"];

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
    #[builder(default = StdIo::Inherit)]
    pub stdin: StdIo,
    #[builder(default = StdIo::Inherit)]
    pub stdout: StdIo,
    #[builder(default = StdIo::Inherit)]
    pub stderr: StdIo,
    // #[builder(default, setter(strip_option))]
    // pub nice: Option<i8>,
    #[builder(default = false)]
    pub background: bool,
}

#[derive(Debug)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StdIo {
    #[default]
    Inherit,
    Null,
}

impl StdIo {
    pub fn valid_string_values() -> &'static [&'static str] {
        VALID_STDIO_VALUES
    }
}

impl TryFrom<&str> for StdIo {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "inherit" | "default" => Ok(StdIo::Inherit),
            "null" | "void" => Ok(StdIo::Null),
            _ => Err(format!("invalid stdio value: {}", value)),
        }
    }
}

impl From<StdIo> for std::process::Stdio {
    fn from(value: StdIo) -> Self {
        match value {
            StdIo::Inherit => std::process::Stdio::inherit(),
            StdIo::Null => std::process::Stdio::null(),
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
