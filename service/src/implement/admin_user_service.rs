use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use common::error::{AppError, AppResult};
use domain::model::admin_user::{AdminRole, AdminUser};
use domain::repository::admin_user::AdminUserRepository;
use shaku::Component;

use crate::services::AdminUserService;

#[derive(Component)]
#[shaku(interface = AdminUserService)]
pub struct AdminUserServiceImpl {
    #[shaku(inject)]
    repo: Arc<dyn AdminUserRepository>,
}

/// 小文字にして前後の空白を除く。メールとして明らかにおかしいものは 400
pub fn normalize_email(email: &str) -> AppResult<String> {
    let e = email.trim().to_lowercase();
    let ok = e.len() <= 255
        && e.split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
        && !e.chars().any(|c| c.is_whitespace() || c.is_control());
    if ok {
        Ok(e)
    } else {
        Err(AppError::BadRequest(
            "メールアドレスの形が正しくありません".to_string(),
        ))
    }
}

impl AdminUserServiceImpl {
    /// `email` が最後の admin なら true(消す・reviewer に下げると、誰も管理できなくなる)
    async fn is_last_admin(&self, email: &str) -> AppResult<bool> {
        let users = self.repo.list_admin_users().await?;
        let admins: Vec<_> = users
            .iter()
            .filter(|u| u.role == AdminRole::Admin)
            .collect();
        Ok(admins.len() == 1 && admins[0].email == email)
    }
}

#[async_trait]
impl AdminUserService for AdminUserServiceImpl {
    async fn list_users(&self) -> AppResult<Vec<AdminUser>> {
        self.repo.list_admin_users().await
    }

    async fn find_user(&self, email: &str) -> AppResult<Option<AdminUser>> {
        self.repo.find_admin_user(email).await
    }

    async fn put_user(&self, email: &str, role: AdminRole) -> AppResult<AdminUser> {
        let email = normalize_email(email)?;
        if role != AdminRole::Admin && self.is_last_admin(&email).await? {
            return Err(AppError::BadRequest(
                "最後の admin の役割は変えられません".to_string(),
            ));
        }
        self.repo
            .upsert_admin_user(&email, role, Utc::now())
            .await?;
        self.repo
            .find_admin_user(&email)
            .await?
            .ok_or_else(|| AppError::EntityNotFound(email))
    }

    async fn delete_user(&self, email: &str, acting_as: Option<&str>) -> AppResult<()> {
        let email = normalize_email(email)?;
        // Access で入っている人が自分を消すと、その場で入れなくなる
        if acting_as.is_some_and(|me| me.eq_ignore_ascii_case(&email)) {
            return Err(AppError::BadRequest("自分自身は消せません".to_string()));
        }
        if self.is_last_admin(&email).await? {
            return Err(AppError::BadRequest(
                "最後の admin は消せません".to_string(),
            ));
        }
        if self.repo.delete_admin_user(&email).await? {
            Ok(())
        } else {
            Err(AppError::EntityNotFound(format!(
                "{email} は登録されていません"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// メモリ上の偽の repository(`domain` の automock は他の crate のテストからは見えない)
    #[derive(Default)]
    struct FakeRepo {
        users: Mutex<Vec<AdminUser>>,
        deletes: Mutex<usize>,
    }

    #[async_trait]
    impl AdminUserRepository for FakeRepo {
        async fn list_admin_users(&self) -> AppResult<Vec<AdminUser>> {
            Ok(self.users.lock().unwrap().clone())
        }
        async fn find_admin_user(&self, email: &str) -> AppResult<Option<AdminUser>> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.email == email)
                .cloned())
        }
        async fn upsert_admin_user(
            &self,
            email: &str,
            role: AdminRole,
            now: chrono::DateTime<Utc>,
        ) -> AppResult<()> {
            let mut users = self.users.lock().unwrap();
            match users.iter_mut().find(|u| u.email == email) {
                Some(u) => u.role = role,
                None => users.push(AdminUser {
                    email: email.to_string(),
                    role,
                    created_at: now,
                }),
            }
            Ok(())
        }
        async fn delete_admin_user(&self, email: &str) -> AppResult<bool> {
            *self.deletes.lock().unwrap() += 1;
            let mut users = self.users.lock().unwrap();
            let before = users.len();
            users.retain(|u| u.email != email);
            Ok(users.len() != before)
        }
    }

    fn svc() -> (AdminUserServiceImpl, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        (AdminUserServiceImpl { repo: repo.clone() }, repo)
    }

    #[test]
    fn normalize_email_cases() {
        assert_eq!(
            normalize_email("  A@Example.COM ").unwrap(),
            "a@example.com"
        );
        for bad in [
            "",
            "a",
            "a@",
            "@example.com",
            "a@example",
            "a b@example.com",
        ] {
            assert!(
                matches!(normalize_email(bad), Err(AppError::BadRequest(_))),
                "{bad}"
            );
        }
    }

    #[tokio::test]
    async fn cannot_delete_self_when_acting_via_access() {
        let (s, repo) = svc();
        s.put_user("me@example.com", AdminRole::Admin)
            .await
            .unwrap();
        let err = s
            .delete_user("Me@Example.com", Some("me@example.com"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
        // 消す処理まで進んでいない
        assert_eq!(*repo.deletes.lock().unwrap(), 0);
        assert_eq!(s.list_users().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn delete_other_and_missing() {
        let (s, _) = svc();
        s.put_user("other@example.com", AdminRole::Reviewer)
            .await
            .unwrap();
        s.delete_user("other@example.com", Some("me@example.com"))
            .await
            .unwrap();
        let err = s.delete_user("other@example.com", None).await.unwrap_err();
        assert!(matches!(err, AppError::EntityNotFound(_)));
    }

    #[tokio::test]
    async fn last_admin_cannot_be_deleted_or_demoted() {
        let (s, _) = svc();
        s.put_user("a@example.com", AdminRole::Admin).await.unwrap();
        s.put_user("r@example.com", AdminRole::Reviewer)
            .await
            .unwrap();
        assert!(matches!(
            s.delete_user("a@example.com", None).await,
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            s.put_user("a@example.com", AdminRole::Reviewer).await,
            Err(AppError::BadRequest(_))
        ));
        // admin が 2 人いれば、どちらも下げられる・消せる
        s.put_user("b@example.com", AdminRole::Admin).await.unwrap();
        s.put_user("a@example.com", AdminRole::Reviewer)
            .await
            .unwrap();
        s.delete_user("b@example.com", None).await.unwrap_err();
        s.put_user("a@example.com", AdminRole::Admin).await.unwrap();
        s.delete_user("b@example.com", None).await.unwrap();
        // reviewer はいつでも消せる
        s.delete_user("r@example.com", None).await.unwrap();
    }

    #[tokio::test]
    async fn put_normalizes_and_changes_role() {
        let (s, _) = svc();
        let u = s
            .put_user(" A@Example.com", AdminRole::Reviewer)
            .await
            .unwrap();
        assert_eq!(u.email, "a@example.com");
        let u = s.put_user("a@example.com", AdminRole::Admin).await.unwrap();
        assert_eq!(u.role, AdminRole::Admin);
        assert_eq!(s.list_users().await.unwrap().len(), 1);
        assert!(matches!(
            s.put_user("bad", AdminRole::Admin).await,
            Err(AppError::BadRequest(_))
        ));
    }
}
