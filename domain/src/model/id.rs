use serde::{Deserialize, Serialize};
use std::str::FromStr;

use common::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Hash, Serialize, Deserialize, sqlx::Type)]
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
