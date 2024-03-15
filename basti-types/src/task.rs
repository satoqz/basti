use std::{fmt::Display, str::FromStr, time::Duration};

use anyhow::bail;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

use crate::WorkerName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub duration: Duration,
    pub priority: TaskPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(flatten)]
    pub key: TaskKey,
    #[serde(flatten)]
    pub value: TaskValue,
}

impl Task {
    pub fn generate(priority: TaskPriority, duration: Duration) -> Self {
        Self {
            key: TaskKey::generate(),
            value: TaskValue::new_with_current_time(duration, priority),
        }
    }
}

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
            _ => bail!("unexpected prefix byte"),
        };

        Ok(Self::new(
            TaskState::try_from(*state)?,
            Uuid::from_slice(rest)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<WorkerName>,
    pub remaining: Duration,
    pub updated_at: DateTime<Utc>,
    pub priority: TaskPriority,
    pub created_at: DateTime<Utc>,
    pub duration: Duration,
}

impl TaskValue {
    pub fn new_with_current_time(duration: Duration, priority: TaskPriority) -> Self {
        let now = Utc::now();
        Self {
            duration,
            remaining: duration,
            created_at: now,
            updated_at: now,
            priority,
            assignee: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
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
            _ => bail!("unexpected state byte"),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskPriority(pub u8);

impl Default for TaskPriority {
    fn default() -> Self {
        Self(10)
    }
}

impl Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TaskPriority {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(Self(s.parse()?))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PriorityKey {
    pub priority: TaskPriority,
    pub id: Uuid,
}

impl PriorityKey {
    pub const PREFIX: u8 = b'p';

    pub fn new(priority: TaskPriority, id: Uuid) -> Self {
        Self { priority, id }
    }
}

impl From<&Task> for PriorityKey {
    fn from(task: &Task) -> Self {
        Self::new(task.value.priority, task.key.id)
    }
}

impl From<&PriorityKey> for Vec<u8> {
    fn from(value: &PriorityKey) -> Self {
        let mut result = vec![PriorityKey::PREFIX, value.priority.0];
        result.extend_from_slice(value.id.as_bytes());
        result
    }
}

impl TryFrom<&[u8]> for PriorityKey {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> anyhow::Result<Self> {
        let (priority, uuid) = match value {
            [Self::PREFIX, priority, uuid @ ..] => (priority, uuid),
            _ => bail!("unexpected prefix byte"),
        };

        Ok(Self::new(TaskPriority(*priority), Uuid::from_slice(uuid)?))
    }
}
