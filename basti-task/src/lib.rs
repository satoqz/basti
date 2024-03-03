mod key;
mod priority;
mod value;

pub use crate::{key::*, priority::*, value::*};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub duration: Duration,
    pub priority: TaskPriority,
}
