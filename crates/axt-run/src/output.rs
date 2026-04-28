use serde::Serialize;

use crate::model::RunData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunOutput {
    Run(RunData),
    Show(RunData),
    Stream {
        name: String,
        stream: String,
        text: String,
    },
    List {
        runs: Vec<ListRun>,
    },
    Clean {
        removed: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListRun {
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub ok: bool,
    pub exit: Option<i32>,
    pub duration_ms: u64,
    pub command: String,
}

impl RunOutput {
    #[must_use]
    pub fn ok(&self) -> bool {
        match self {
            Self::Run(data) | Self::Show(data) => data.ok(),
            Self::Stream { .. } | Self::List { .. } | Self::Clean { .. } => true,
        }
    }
}
