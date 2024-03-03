use crate::task::{Task, TaskPriority};
use std::fmt::Display;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct PriorityKey {
    priority: TaskPriority,
    id: Uuid,
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
