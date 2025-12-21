//! マルチパートファイルアップロードのヘルパー関数

use axum::extract::Multipart;
use common::error::{AppError, AppResult};

/// マルチパートからUTF-8文字列データを抽出する
pub async fn extract_text_file(multipart: &mut Multipart) -> AppResult<String> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| {
            AppError::UnprocessableEntity(format!("マルチパートの読み込みに失敗しました: {}", e))
        })?
        .ok_or_else(|| {
            AppError::UnprocessableEntity("ファイルが送信されていません".to_string())
        })?;

    let data = field.bytes().await.map_err(|e| {
        AppError::UnprocessableEntity(format!("ファイルの読み込みに失敗しました: {}", e))
    })?;

    String::from_utf8(data.to_vec())
        .map_err(|_| AppError::UnprocessableEntity("ファイルがUTF-8形式ではありません".to_string()))
}
