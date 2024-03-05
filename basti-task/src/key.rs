use anyhow::bail;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TaskKey {
    pub state: TaskState,
    pub id: Uuid,
}

impl TaskKey {
    pub const PREFIX: u8 = b't';

    pub fn new(state: TaskState, id: Uuid) -> Self {
        Self { state, id }
    }

    pub fn generate() -> Self {
        Self {
            state: TaskState::default(),
            id: Uuid::new_v4(),
        }
    }
}

impl From<&TaskKey> for Vec<u8> {
    fn from(value: &TaskKey) -> Self {
        let mut result = vec![TaskKey::PREFIX, value.state.into()];
        result.extend_from_slice(value.id.as_bytes());
        result
    }
}

impl TryFrom<&[u8]> for TaskKey {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> anyhow::Result<Self> {
        let (state, rest) = match value {
            [Self::PREFIX, state, rest @ ..] => (state, rest),
            _ => bail!("Wrong prefix byte"),
        };

        Ok(Self::new(
            TaskState::try_from(*state)?,
            Uuid::from_slice(rest)?,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
}

impl TaskState {
    pub const VARIANTS: [Self; 2] = [Self::Queued, Self::Running];
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Queued
    }
}

impl From<TaskState> for u8 {
    fn from(value: TaskState) -> Self {
        match value {
            TaskState::Queued => b'q',
            TaskState::Running => b'r',
        }
    }
}

impl TryFrom<u8> for TaskState {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> anyhow::Result<Self> {
        Ok(match value {
            b'q' => Self::Queued,
            b'r' => Self::Running,
            _ => bail!("Invalid TaskState byte"),
        })
    }
}
