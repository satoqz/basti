use crate::{Task, TaskPriority};
use anyhow::bail;
use uuid::Uuid;

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
            _ => bail!("Wrong prefix byte"),
        };

        Ok(Self::new(TaskPriority(*priority), Uuid::from_slice(uuid)?))
    }
}
