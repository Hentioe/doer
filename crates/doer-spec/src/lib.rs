use std::collections::HashSet;

#[derive(Debug)]
pub struct Runnable {
    pub name: String,
    pub commands: Vec<String>,
    pub cwd: Option<String>,
    pub env_vars: HashSet<EnvVar>,
    pub user: Option<String>,
    pub background: bool,
}

#[derive(Debug)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

impl PartialEq for EnvVar {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for EnvVar {}

impl std::hash::Hash for EnvVar {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
