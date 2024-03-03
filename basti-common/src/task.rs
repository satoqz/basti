use anyhow::{anyhow, bail, Error, Result};
use chrono::{DateTime, Utc};
use clap::ValueEnum;
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

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskDetails {
    pub priority: u32,
    pub remaining: Duration,
    pub created_at: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskPayload {
    pub duration: Duration,
    #[serde(default)]
    pub priority: u32,
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
        let now = Utc::now();
        Self {
            priority,
            assignee: None,
            duration,
            remaining: duration,
            created_at: now,
            last_update: now,
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

impl FromStr for TaskKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('_').collect();

        if parts.len() != 3 || parts[0] != "task" {
            bail!("malformed task key")
        }

        let state = TaskState::from_str(parts[1], false).map_err(|err| anyhow!(err))?;
        let id = Uuid::from_str(parts[2])?;

        Ok(Self { id, state })
    }
}
