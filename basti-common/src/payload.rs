use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub duration: Duration,
    pub priority: u8,
}
