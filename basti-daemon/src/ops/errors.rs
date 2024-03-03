#[derive(Debug)]
pub enum MaybeRevisionError {
    BadRevision,
    Other(anyhow::Error),
}

impl From<anyhow::Error> for MaybeRevisionError {
    fn from(value: anyhow::Error) -> Self {
        Self::Other(value)
    }
}
