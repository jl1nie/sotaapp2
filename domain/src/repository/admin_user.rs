use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

use crate::model::admin_user::{AdminRole, AdminUser};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait AdminUserRepository: Send + Sync + Interface {
    async fn list_admin_users(&self) -> AppResult<Vec<AdminUser>>;
    async fn find_admin_user(&self, email: &str) -> AppResult<Option<AdminUser>>;
    /// 足す、または役割を変える(`created_at` は足したときだけ入る)
    async fn upsert_admin_user(
        &self,
        email: &str,
        role: AdminRole,
        now: DateTime<Utc>,
    ) -> AppResult<()>;
    /// 消した行があれば true
    async fn delete_admin_user(&self, email: &str) -> AppResult<bool>;
}
