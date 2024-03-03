use anyhow::{bail, Error};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

pub struct PointerKey(Uuid);

impl PointerKey {
    pub fn prefix() -> &'static str {
        "pointer"
    }
}

impl Display for PointerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", Self::prefix(), self.0)
    }
}

impl FromStr for PointerKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('_').collect();

        if parts.len() != 2 || parts[0] != Self::prefix() {
            bail!("malformed key")
        }

        Ok(Self(Uuid::from_str(parts[1])?))
    }
}
