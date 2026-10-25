use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shaku::Component;
use sqlx::Row;

use common::error::{db_error, AppResult};
use domain::model::admin_user::{AdminRole, AdminUser};
use domain::repository::admin_user::AdminUserRepository;

use crate::database::connect::ConnectionPool;

#[derive(Component)]
#[shaku(interface = AdminUserRepository)]
pub struct AdminUserRepositoryImpl {
    pool: ConnectionPool,
}

fn to_admin_user(row: &sqlx::sqlite::SqliteRow) -> Option<AdminUser> {
    let role = AdminRole::parse(&row.get::<String, _>("role"))?;
    Some(AdminUser {
        email: row.get("email"),
        role,
        created_at: DateTime::from_timestamp(row.get::<i64, _>("created_at"), 0)
            .unwrap_or_default(),
    })
}

#[async_trait]
impl AdminUserRepository for AdminUserRepositoryImpl {
    async fn list_admin_users(&self) -> AppResult<Vec<AdminUser>> {
        let rows = sqlx::query("SELECT email, role, created_at FROM admin_users ORDER BY email")
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(db_error("select admin_users"))?;
        // 知らない役割の行は、権限を与えないよう読み飛ばす
        Ok(rows.iter().filter_map(to_admin_user).collect())
    }

    async fn find_admin_user(&self, email: &str) -> AppResult<Option<AdminUser>> {
        let row = sqlx::query("SELECT email, role, created_at FROM admin_users WHERE email = $1")
            .bind(email.to_lowercase())
            .fetch_optional(self.pool.inner_ref())
            .await
            .map_err(db_error("select admin_user"))?;
        Ok(row.as_ref().and_then(to_admin_user))
    }

    async fn upsert_admin_user(
        &self,
        email: &str,
        role: AdminRole,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO admin_users(email, role, created_at) VALUES($1, $2, $3)
            ON CONFLICT (email) DO UPDATE SET role = EXCLUDED.role
        "#,
        )
        .bind(email.to_lowercase())
        .bind(role.as_str())
        .bind(now.timestamp())
        .execute(self.pool.inner_ref())
        .await
        .map_err(db_error("upsert admin_user"))?;
        Ok(())
    }

    async fn delete_admin_user(&self, email: &str) -> AppResult<bool> {
        let r = sqlx::query("DELETE FROM admin_users WHERE email = $1")
            .bind(email.to_lowercase())
            .execute(self.pool.inner_ref())
            .await
            .map_err(db_error("delete admin_user"))?;
        Ok(r.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::Migrator;
    use sqlx::sqlite::SqlitePool;
    use std::path::Path;
    use tempfile::tempdir;

    async fn repo() -> (AdminUserRepositoryImpl, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        std::fs::File::create(&path).unwrap();
        let pool = SqlitePool::connect(&format!("sqlite:{}", path.display()))
            .await
            .unwrap();
        Migrator::new(Path::new("migrations/sqlite"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        (
            AdminUserRepositoryImpl {
                pool: ConnectionPool::new(pool),
            },
            dir,
        )
    }

    #[tokio::test]
    async fn upsert_find_list_delete() {
        let (r, _d) = repo().await;
        let t0 = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let t1 = DateTime::from_timestamp(1_700_000_100, 0).unwrap();
        assert!(r.find_admin_user("a@example.com").await.unwrap().is_none());

        r.upsert_admin_user("A@Example.com", AdminRole::Reviewer, t0)
            .await
            .unwrap();
        // 役割を変えても created_at は変わらない。メールは大文字小文字を区別しない
        r.upsert_admin_user("a@example.com", AdminRole::Admin, t1)
            .await
            .unwrap();
        let u = r.find_admin_user("A@EXAMPLE.COM").await.unwrap().unwrap();
        assert_eq!(u.email, "a@example.com");
        assert_eq!(u.role, AdminRole::Admin);
        assert_eq!(u.created_at, t0);
        assert_eq!(r.list_admin_users().await.unwrap().len(), 1);

        assert!(r.delete_admin_user("a@example.com").await.unwrap());
        assert!(!r.delete_admin_user("a@example.com").await.unwrap());
        assert!(r.list_admin_users().await.unwrap().is_empty());
    }
}
