use serde::Serialize;

use crate::model::DriftData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DriftOutput {
    Mark(DriftData),
    Diff(DriftData),
    Run(DriftData),
    List(DriftData),
    Reset(DriftData),
}

impl DriftOutput {
    #[must_use]
    pub const fn data(&self) -> &DriftData {
        match self {
            Self::Mark(data)
            | Self::Diff(data)
            | Self::Run(data)
            | Self::List(data)
            | Self::Reset(data) => data,
        }
    }

    #[must_use]
    pub fn ok(&self) -> bool {
        self.data().ok()
    }
}
