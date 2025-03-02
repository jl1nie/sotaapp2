use common::error::AppError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(uuid::Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
    pub fn raw(self) -> uuid::Uuid {
        self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        UserId::new()
    }
}

impl FromStr for UserId {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(uuid::Uuid::parse_str(s)?))
    }
}

impl From<uuid::Uuid> for UserId {
    fn from(value: uuid::Uuid) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct LogId(uuid::Uuid);

impl LogId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
    pub fn raw(self) -> uuid::Uuid {
        self.0
    }
}

impl Default for LogId {
    fn default() -> Self {
        LogId::new()
    }
}

impl FromStr for LogId {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(uuid::Uuid::parse_str(s)?))
    }
}

impl From<uuid::Uuid> for LogId {
    fn from(value: uuid::Uuid) -> Self {
        Self(value)
    }
}

impl From<LogId> for String {
    fn from(value: LogId) -> Self {
        value.0.to_string()
    }
}

impl fmt::Display for LogId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
