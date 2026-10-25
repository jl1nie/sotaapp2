use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

/// 署名の検証を通った、Cloudflare Access のログインの人
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessIdentity {
    /// 小文字
    pub email: String,
}

/// Cloudflare Access が付ける署名(JWT)を検証するポート
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AccessTokenVerifier: Send + Sync + Interface {
    /// 設定(チームのドメインと AUD)がそろっていて、検証できるか
    fn is_enabled(&self) -> bool;
    /// 署名・aud・iss・exp を確かめて、メールを取り出す。通らなければ `UnauthenticatedError`
    async fn verify(&self, token: &str) -> AppResult<AccessIdentity>;
}
