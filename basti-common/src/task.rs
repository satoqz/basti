use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr, time::Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(flatten)]
    pub key: TaskKey,
    #[serde(flatten)]
    pub details: TaskDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskKey {
    pub id: Uuid,
    pub state: TaskState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetails {
    pub priority: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub duration: Duration,
    pub remaining: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskPayload {
    pub duration: Duration,
    #[serde(default)]
    pub priority: u32,
}

pub enum Error {
    UnknownState,
    MalformedKey,
}

impl Task {
    pub fn generate(priority: u32, duration: Duration) -> Self {
        Self {
            key: TaskKey::default(),
            details: TaskDetails::new(priority, duration),
        }
    }
}

impl Default for TaskKey {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            state: TaskState::default(),
        }
    }
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Queued
    }
}

impl TaskDetails {
    pub fn new(priority: u32, duration: Duration) -> Self {
        Self {
            priority,
            duration,
            remaining: duration,
            assignee: None,
        }
    }
}

impl Display for TaskKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task_{}_{}", self.state, self.id)
    }
}

impl Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownState => write!(f, "unknown task state"),
            Self::MalformedKey => write!(f, "malformed task key"),
        }
    }
}

impl FromStr for TaskKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('_').collect();

        if parts.len() != 3 || parts[0] != "task" {
            return Err(Error::MalformedKey);
        }

        let state = TaskState::from_str(parts[1])?;
        let id = Uuid::from_str(parts[2]).map_err(|_| Error::MalformedKey)?;

        Ok(Self { id, state })
    }
}

impl FromStr for TaskState {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            _ => Err(Error::UnknownState),
        }
    }
}
