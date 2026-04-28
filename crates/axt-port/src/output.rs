use serde::Serialize;

use crate::model::PortData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PortOutput {
    List(PortData),
    Who(PortData),
    Free(PortData),
    Watch(PortData),
}

impl PortOutput {
    #[must_use]
    pub const fn data(&self) -> &PortData {
        match self {
            Self::List(data) | Self::Who(data) | Self::Free(data) | Self::Watch(data) => data,
        }
    }

    #[must_use]
    pub fn ok(&self) -> bool {
        self.data().ok()
    }
}
