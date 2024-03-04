use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use strum_macros::{Display as StrumDisplay, EnumString, VariantArray};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskKey {
    pub state: TaskState,
    pub id: Uuid,
}

impl TaskKey {
    pub fn generate() -> Self {
        Self {
            state: TaskState::default(),
            id: Uuid::new_v4(),
        }
    }

    pub fn prefix() -> &'static str {
        "t"
    }
}

impl Display for TaskKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", Self::prefix(), self.state, self.id)
    }
}

impl FromStr for TaskKey {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = s.split('_').collect();

        if parts.len() != 3 || parts[0] != Self::prefix() {
            bail!("Malformed key")
        }

        Ok(Self {
            state: TaskState::from_str(parts[1])?,
            id: Uuid::from_str(parts[2])?,
        })
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    EnumString,
    VariantArray,
    StrumDisplay,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Queued
    }
}
