use std::{fmt::Display, str::FromStr};

use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Name(String);

impl Name {
    pub fn validate(s: &str) -> anyhow::Result<()> {
        if s.len() < 1 {
            bail!("name is empty")
        }

        if s.len() > 32 {
            bail!("name is longer than 32 characters")
        }

        if !s.chars().all(|c| {
            c == '-' || c.is_ascii_digit() || (c.is_ascii_alphabetic() && c.is_ascii_lowercase())
        }) {
            bail!("name may only contain characters -, 0-9, a-z")
        }

        Ok(())
    }
}

impl FromStr for Name {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::validate(s).map(|_| Self(s.to_string()))
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::from_str(name.as_ref())
            .map(|_| Self(name))
            .map_err(|err| serde::de::Error::custom(err))
    }
}
