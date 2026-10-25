use chrono::{DateTime, Utc};

/// 管理画面での役割
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminRole {
    /// すべての管理機能
    Admin,
    /// 見直し(山頂の地図の目視)だけ
    Reviewer,
}

impl AdminRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Reviewer => "reviewer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Self::Admin),
            "reviewer" => Some(Self::Reviewer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdminUser {
    /// 小文字
    pub email: String,
    pub role: AdminRole,
    pub created_at: DateTime<Utc>,
}
