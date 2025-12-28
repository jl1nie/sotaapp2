//! OpenAPI ドキュメント生成モジュール

use common::config::OpenApiLevel;
use utoipa::openapi::OpenApi;

use crate::handler::health::HealthApi;
use crate::handler::search::SearchApi;

/// SOTAApp2 API ドキュメントを生成（レベル別）
///
/// - `None`: ドキュメント生成なし（空のOpenApi）
/// - `Public`: 公開API（認証不要のエンドポイントのみ）
/// - `All`: すべてのAPI（admin系含む）
pub fn create_api_doc(level: OpenApiLevel) -> Option<OpenApi> {
    match level {
        OpenApiLevel::None => None,
        OpenApiLevel::Public => Some(create_public_api_doc()),
        OpenApiLevel::All => Some(create_full_api_doc()),
    }
}

/// 公開API用ドキュメント（認証不要のエンドポイントのみ）
fn create_public_api_doc() -> OpenApi {
    use utoipa::OpenApi;

    let mut doc = HealthApi::openapi();
    doc.merge(SearchApi::openapi());

    set_api_info(&mut doc, "SOTAApp2 Public API");
    doc
}

/// 全API用ドキュメント（admin系含む）
fn create_full_api_doc() -> OpenApi {
    use utoipa::OpenApi;

    let mut doc = HealthApi::openapi();
    doc.merge(SearchApi::openapi());
    // TODO: 将来的にAdminApiを追加

    set_api_info(&mut doc, "SOTAApp2 API");
    doc
}

/// API情報を設定
fn set_api_info(doc: &mut OpenApi, title: &str) {
    doc.info.title = title.to_string();
    doc.info.version = "2.0.0".to_string();
    doc.info.description = Some("アマチュア無線アワードプログラム（SOTA/POTA）管理API".to_string());
}

/// OpenAPI仕様を取得するための型エイリアス
pub type ApiDoc = OpenApi;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_api_doc_none_returns_none() {
        let result = create_api_doc(OpenApiLevel::None);
        assert!(result.is_none());
    }

    #[test]
    fn test_create_api_doc_public_returns_some() {
        let result = create_api_doc(OpenApiLevel::Public);
        assert!(result.is_some());

        let doc = result.unwrap();
        assert_eq!(doc.info.title, "SOTAApp2 Public API");
        assert_eq!(doc.info.version, "2.0.0");
        assert!(doc.info.description.is_some());
    }

    #[test]
    fn test_create_api_doc_all_returns_some() {
        let result = create_api_doc(OpenApiLevel::All);
        assert!(result.is_some());

        let doc = result.unwrap();
        assert_eq!(doc.info.title, "SOTAApp2 API");
        assert_eq!(doc.info.version, "2.0.0");
    }

    #[test]
    fn test_public_doc_contains_health_paths() {
        let doc = create_api_doc(OpenApiLevel::Public).unwrap();
        let paths = doc.paths.paths;

        assert!(paths.contains_key("/api/v2/health"));
        assert!(paths.contains_key("/api/v2/health/db"));
    }

    #[test]
    fn test_public_doc_contains_search_paths() {
        let doc = create_api_doc(OpenApiLevel::Public).unwrap();
        let paths = doc.paths.paths;

        assert!(paths.contains_key("/api/v2/search"));
        assert!(paths.contains_key("/api/v2/search/full"));
        assert!(paths.contains_key("/api/v2/search/brief"));
    }
}
