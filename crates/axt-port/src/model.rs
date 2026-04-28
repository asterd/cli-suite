use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PortData {
    pub action: PortAction,
    pub ports: Vec<u16>,
    pub sockets: Vec<SocketEntry>,
    pub holders: Vec<PortHolder>,
    pub attempts: Vec<FreeAttempt>,
    pub held: bool,
    pub freed: bool,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub truncated: bool,
}

impl PortData {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.attempts.iter().all(|attempt| attempt.ok) && !self.timed_out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PortAction {
    List,
    Who,
    Free,
    Watch,
}

impl PortAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Who => "who",
            Self::Free => "free",
            Self::Watch => "watch",
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SocketEntry {
    pub port: u16,
    pub proto: Protocol,
    pub pid: Option<u32>,
    pub process: Option<String>,
    pub bound: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PortHolder {
    pub port: u16,
    pub proto: Protocol,
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub bound: Vec<String>,
    pub owner: Option<String>,
    pub memory_bytes: Option<u64>,
    pub started: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FreeAttempt {
    pub port: u16,
    pub pid: u32,
    pub name: String,
    pub signal: String,
    pub action: FreeAction,
    pub result: FreeResult,
    pub ok: bool,
    pub escalated: bool,
    pub ms: u64,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FreeAction {
    Signaled,
    Simulated,
    Refused,
}

impl FreeAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Signaled => "signaled",
            Self::Simulated => "simulated",
            Self::Refused => "refused",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FreeResult {
    Freed,
    Held,
    Skipped,
    PermissionDenied,
    Refused,
    Failed,
}

impl FreeResult {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Freed => "freed",
            Self::Held => "held",
            Self::Skipped => "skipped",
            Self::PermissionDenied => "permission_denied",
            Self::Refused => "refused",
            Self::Failed => "failed",
        }
    }
}
