use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

const VALID_STDIO_VALUES: &[&str] = &["default", "inherit", "null", "void"];

#[derive(Debug)]
pub struct Runnable {
    pub name: String,
    pub commands: Vec<String>,
    pub cwd: Option<String>,
    pub env_vars: HashSet<EnvVar>,
    pub user: Option<String>,
    pub stdin: StdIo,
    pub stdout: StdIo,
    pub stderr: StdIo,
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
