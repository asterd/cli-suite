use serde::Serialize;

use crate::model::DocData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocOutput {
    Which(DocData),
    Path(DocData),
    Env(DocData),
    All(DocData),
}

impl DocOutput {
    #[must_use]
    pub const fn data(&self) -> &DocData {
        match self {
            Self::Which(data) | Self::Path(data) | Self::Env(data) | Self::All(data) => data,
        }
    }
}
