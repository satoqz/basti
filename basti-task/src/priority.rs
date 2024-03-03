use crate::{Task, TaskPriority};
use anyhow::{bail, Error, Result};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct PriorityKey {
    pub priority: TaskPriority,
    pub id: Uuid,
}

impl PriorityKey {
    pub fn prefix() -> &'static str {
        "priority"
    }
}

impl Display for PriorityKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{:03}_{}", Self::prefix(), self.priority, self.id)
    }
}

impl From<&Task> for PriorityKey {
    fn from(task: &Task) -> Self {
        Self {
            priority: task.value.priority,
            id: task.key.id,
        }
    }
}

impl FromStr for PriorityKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('_').collect();

        if parts.len() != 3 || parts[0] != Self::prefix() {
            bail!("Malformed key")
        }

        Ok(Self {
            priority: TaskPriority::from_str(parts[1])?,
            id: Uuid::from_str(parts[2])?,
        })
    }
}
