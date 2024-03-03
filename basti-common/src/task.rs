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
pub struct TaskKey {
    pub state: TaskState,
    pub priority: TaskPriority,
    pub id: Uuid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskPriority(pub u8);

impl Task {
    pub fn new(priority: TaskPriority, duration: Duration) -> Self {
        Self {
            key: TaskKey::new(priority),
            value: TaskValue::new(duration),
        }
    }
}

impl TaskKey {
    pub fn new(priority: TaskPriority) -> Self {
        Self {
            state: TaskState::default(),
            priority,
            id: Uuid::new_v4(),
        }
    }

    pub fn prefix() -> &'static str {
        "task"
    }
}

impl Display for TaskKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}_{}_{:03}_{}",
            Self::prefix(),
            self.state,
            self.priority,
            self.id
        )
    }
}

impl FromStr for TaskKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('_').collect();

        if parts.len() != 4 || parts[0] != Self::prefix() {
            bail!("malformed key")
        }

        let state = TaskState::from_str(parts[1], false).map_err(|err| anyhow!(err))?;
        let priority = TaskPriority::from_str(parts[2])?;
        let id = Uuid::from_str(parts[3])?;

        Ok(Self {
            state,
            priority,
            id,
        })
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

impl Default for TaskState {
    fn default() -> Self {
        Self::Queued
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
    type Err = Error;
    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}
