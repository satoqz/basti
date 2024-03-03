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
    pub value: TaskValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskKey {
    pub state: TaskState,
    pub priority: u8,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskValue {
    pub duration: Duration,
    pub remaining: Duration,
    pub created_at: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskPayload {
    pub duration: Duration,
    pub priority: u8,
}

impl Task {
    pub fn new(priority: u8, duration: Duration) -> Self {
        Self {
            key: TaskKey::new(priority),
            value: TaskValue::new(duration),
        }
    }
}

impl TaskKey {
    fn new(priority: u8) -> Self {
        Self {
            state: TaskState::default(),
            priority,
            id: Uuid::new_v4(),
        }
    }
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Queued
    }
}

impl TaskValue {
    pub fn new(duration: Duration) -> Self {
        let now = Utc::now();
        Self {
            duration,
            remaining: duration,
            created_at: now,
            last_update: now,
            assignee: None,
        }
    }
}

impl Display for TaskKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task_{}_{:03}_{}", self.state, self.priority, self.id)
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

        if parts.len() != 4 || parts[0] != "task" {
            bail!("malformed task key")
        }

        let state = TaskState::from_str(parts[1], false).map_err(|err| anyhow!(err))?;
        let priority = parts[2].parse()?;
        let id = Uuid::from_str(parts[3])?;

        Ok(Self {
            state,
            priority,
            id,
        })
    }
}
