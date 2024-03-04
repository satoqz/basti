use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskValue {
    pub duration: Duration,
    pub remaining: Duration,
    pub created_at: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    pub priority: TaskPriority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
}

impl TaskValue {
    pub fn new_with_current_time(duration: Duration, priority: TaskPriority) -> Self {
        let now = Utc::now();
        Self {
            duration,
            remaining: duration,
            created_at: now,
            last_update: now,
            priority,
            assignee: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
