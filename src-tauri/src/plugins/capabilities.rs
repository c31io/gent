use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Context,
    Tools,
    Memory,
    Nodes,
    Execution,
}

impl Capability {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "context" => Some(Self::Context),
            "tools" => Some(Self::Tools),
            "memory" => Some(Self::Memory),
            "nodes" => Some(Self::Nodes),
            "execution" => Some(Self::Execution),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Tools => "tools",
            Self::Memory => "memory",
            Self::Nodes => "nodes",
            Self::Execution => "execution",
        }
    }
}
