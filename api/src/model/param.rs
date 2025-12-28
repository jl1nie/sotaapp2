use axum::{
    extract::{FromRequestParts, Query},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use common::error::{AppError, AppResult};
use domain::model::{
    event::{FindRef, FindRefBuilder},
    id::LogId,
};

/// バリデーションエラーレスポンス
#[derive(Debug, Serialize)]
pub struct ValidationErrorResponse {
    pub success: bool,
    pub message: String,
    pub code: String,
}

/// バリデーション付きクエリパラメータエクストラクタ
///
/// Axumの`Query`エクストラクタと同様に使用できますが、
/// 自動的に`Validate`トレイトによるバリデーションを実行します。
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedQuery<T>(pub T);

impl<T> ValidatedQuery<T> {
    /// 内部の値を取り出す
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate + Send,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // まずQueryエクストラクタでデシリアライズ
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                let body = ValidationErrorResponse {
                    success: false,
                    message: format!("クエリパラメータの解析に失敗しました: {}", e),
                    code: "INVALID_QUERY".to_string(),
                };
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            })?;

        // バリデーション実行
        value.validate().map_err(|e| {
            let messages: Vec<String> = e
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |err| {
                        err.message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| format!("{}: 無効な値です", field))
                    })
                })
                .collect();

            let body = ValidationErrorResponse {
                success: false,
                message: messages.join(", "),
                code: "VALIDATION_ERROR".to_string(),
            };
            (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
        })?;

        Ok(ValidatedQuery(value))
    }
}

// バリデーション制約値は#[validate]属性内で直接指定
// 定数として定義するとvalidatorマクロが対応しないため

/// APIクエリパラメータ
#[derive(Debug, Clone, Default, Deserialize, Serialize, Validate, ToSchema, IntoParams)]
pub struct GetParam {
    #[validate(range(min = -180.0, max = 180.0, message = "経度は-180〜180の範囲で指定してください"))]
    pub lon: Option<f64>,
    #[validate(range(min = -90.0, max = 90.0, message = "緯度は-90〜90の範囲で指定してください"))]
    pub lat: Option<f64>,
    #[validate(range(
        min = 0.0,
        max = 500.0,
        message = "距離は0〜500kmの範囲で指定してください"
    ))]
    pub dist: Option<f64>,
    #[validate(range(min = -180.0, max = 180.0, message = "経度は-180〜180の範囲で指定してください"))]
    pub min_lon: Option<f64>,
    #[validate(range(min = -90.0, max = 90.0, message = "緯度は-90〜90の範囲で指定してください"))]
    pub min_lat: Option<f64>,
    #[validate(range(min = -180.0, max = 180.0, message = "経度は-180〜180の範囲で指定してください"))]
    pub max_lon: Option<f64>,
    #[validate(range(min = -90.0, max = 90.0, message = "緯度は-90〜90の範囲で指定してください"))]
    pub max_lat: Option<f64>,
    #[validate(range(
        min = 0,
        max = 9000,
        message = "標高は0〜9000mの範囲で指定してください"
    ))]
    pub min_elev: Option<i32>,
    #[validate(range(
        min = 0,
        max = 100000000,
        message = "面積は0〜100000000の範囲で指定してください"
    ))]
    pub min_area: Option<i32>,
    #[validate(range(
        min = 1,
        max = 10000,
        message = "max_countは1〜10000の範囲で指定してください"
    ))]
    pub max_count: Option<u32>,
    #[validate(length(max = 20, message = "pota_codeは20文字以内で指定してください"))]
    pub pota_code: Option<String>,
    #[validate(length(max = 20, message = "sota_codeは20文字以内で指定してください"))]
    pub sota_code: Option<String>,
    #[validate(length(max = 20, message = "wwff_codeは20文字以内で指定してください"))]
    pub wwff_code: Option<String>,
    #[validate(length(max = 100, message = "user_idは100文字以内で指定してください"))]
    pub user_id: Option<String>,
    #[validate(length(max = 50, message = "log_idは50文字以内で指定してください"))]
    pub log_id: Option<String>,
    #[validate(length(max = 100, message = "nameは100文字以内で指定してください"))]
    pub name: Option<String>,
    #[validate(range(
        min = 0,
        max = 8760,
        message = "hours_agoは0〜8760（1年）の範囲で指定してください"
    ))]
    pub hours_ago: Option<i64>,
    #[validate(range(
        min = 1,
        max = 10000,
        message = "limitは1〜10000の範囲で指定してください"
    ))]
    pub limit: Option<i32>,
    #[validate(range(
        min = 0,
        max = 1000000,
        message = "offsetは0〜1000000の範囲で指定してください"
    ))]
    pub offset: Option<i32>,
    #[validate(range(
        min = 1000,
        max = 999999,
        message = "muni_codeは1000〜999999の範囲で指定してください"
    ))]
    pub muni_code: Option<i32>,
    #[validate(length(max = 20, message = "by_callは20文字以内で指定してください"))]
    pub by_call: Option<String>,
    #[validate(length(max = 20, message = "by_refは20文字以内で指定してください"))]
    pub by_ref: Option<String>,
    #[validate(length(max = 50, message = "pat_refは50文字以内で指定してください"))]
    pub pat_ref: Option<String>,
}

impl GetParam {
    pub fn to_key(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }

    /// バリデーションを実行し、エラーがあればAppErrorを返す
    pub fn validated(self) -> AppResult<Self> {
        self.validate().map_err(|e| {
            let messages: Vec<String> = e
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |err| {
                        err.message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| format!("{}: 無効な値です", field))
                    })
                })
                .collect();
            AppError::UnprocessableEntity(messages.join(", "))
        })?;
        Ok(self)
    }
}

pub fn build_findref_query(param: GetParam, mut query: FindRefBuilder) -> AppResult<FindRef> {
    if param.limit.is_some() {
        query = query.limit(param.limit.unwrap());
    }

    if param.offset.is_some() {
        query = query.offset(param.offset.unwrap());
    }

    if param.name.is_some() {
        query = query.name(param.name.unwrap());
    }

    if param.sota_code.is_some() {
        query = query.sota_code(param.sota_code.unwrap());
    }

    if param.pota_code.is_some() {
        query = query.pota_code(param.pota_code.unwrap());
    }

    if param.wwff_code.is_some() {
        query = query.wwff_code(param.wwff_code.unwrap());
    }

    if param.log_id.is_some() {
        if let Ok(log_id) = LogId::from_str(&param.log_id.unwrap()) {
            query = query.log_id(log_id);
        }
    }

    if param.min_area.is_some() {
        query = query.min_area(param.min_area.unwrap());
    }

    if param.min_elev.is_some() {
        query = query.min_elev(param.min_elev.unwrap());
    }

    if param.max_lat.is_some()
        && param.min_lat.is_some()
        && param.max_lon.is_some()
        && param.min_lon.is_some()
    {
        query = query.bbox(
            param.min_lon.unwrap(),
            param.min_lat.unwrap(),
            param.max_lon.unwrap(),
            param.max_lat.unwrap(),
        );
    } else if param.dist.is_some() && param.lon.is_some() && param.lat.is_some() {
        query = query.center(param.lon.unwrap(), param.lat.unwrap(), param.dist.unwrap());
    }

    Ok(query.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::model::event::FindRefBuilder;

    // =====================================================
    // GetParam バリデーションテスト
    // =====================================================

    /// 正常な座標値のバリデーション
    #[test]
    fn test_valid_coordinates() {
        let param = GetParam {
            lon: Some(139.7),
            lat: Some(35.6),
            dist: Some(10.0),
            ..Default::default()
        };
        assert!(param.validated().is_ok());
    }

    /// 経度の範囲外値でエラー
    #[test]
    fn test_invalid_longitude_too_high() {
        let param = GetParam {
            lon: Some(181.0),
            ..Default::default()
        };
        let result = param.validated();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("経度"));
    }

    #[test]
    fn test_invalid_longitude_too_low() {
        let param = GetParam {
            lon: Some(-181.0),
            ..Default::default()
        };
        assert!(param.validated().is_err());
    }

    /// 緯度の範囲外値でエラー
    #[test]
    fn test_invalid_latitude_too_high() {
        let param = GetParam {
            lat: Some(91.0),
            ..Default::default()
        };
        let result = param.validated();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("緯度"));
    }

    #[test]
    fn test_invalid_latitude_too_low() {
        let param = GetParam {
            lat: Some(-91.0),
            ..Default::default()
        };
        assert!(param.validated().is_err());
    }

    /// 距離の範囲外値でエラー
    #[test]
    fn test_invalid_distance_too_high() {
        let param = GetParam {
            dist: Some(501.0),
            ..Default::default()
        };
        let result = param.validated();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("距離"));
    }

    #[test]
    fn test_invalid_distance_negative() {
        let param = GetParam {
            dist: Some(-1.0),
            ..Default::default()
        };
        assert!(param.validated().is_err());
    }

    /// hours_agoの範囲テスト
    #[test]
    fn test_valid_hours_ago() {
        let param = GetParam {
            hours_ago: Some(24),
            ..Default::default()
        };
        assert!(param.validated().is_ok());
    }

    #[test]
    fn test_invalid_hours_ago_too_high() {
        let param = GetParam {
            hours_ago: Some(8761), // > 8760 (1年)
            ..Default::default()
        };
        let result = param.validated();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("hours_ago"));
    }

    /// limitの範囲テスト
    #[test]
    fn test_valid_limit() {
        let param = GetParam {
            limit: Some(100),
            ..Default::default()
        };
        assert!(param.validated().is_ok());
    }

    #[test]
    fn test_invalid_limit_zero() {
        let param = GetParam {
            limit: Some(0),
            ..Default::default()
        };
        assert!(param.validated().is_err());
    }

    #[test]
    fn test_invalid_limit_too_high() {
        let param = GetParam {
            limit: Some(10001),
            ..Default::default()
        };
        assert!(param.validated().is_err());
    }

    /// 文字列長のバリデーション
    #[test]
    fn test_valid_string_length() {
        let param = GetParam {
            pota_code: Some("JA-0001".to_string()),
            sota_code: Some("JA/TK-001".to_string()),
            ..Default::default()
        };
        assert!(param.validated().is_ok());
    }

    #[test]
    fn test_invalid_string_too_long() {
        let param = GetParam {
            pota_code: Some("A".repeat(21)), // > 20文字
            ..Default::default()
        };
        assert!(param.validated().is_err());
    }

    /// 空のパラメータは有効
    #[test]
    fn test_empty_params_valid() {
        let param = GetParam::default();
        assert!(param.validated().is_ok());
    }

    // =====================================================
    // GetParam::to_key テスト
    // =====================================================

    #[test]
    fn test_to_key_deterministic() {
        let param = GetParam {
            lon: Some(139.7),
            lat: Some(35.6),
            ..Default::default()
        };
        let key1 = param.to_key();
        let key2 = param.to_key();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_to_key_different_params() {
        let param1 = GetParam {
            lon: Some(139.7),
            ..Default::default()
        };
        let param2 = GetParam {
            lon: Some(140.0),
            ..Default::default()
        };
        assert_ne!(param1.to_key(), param2.to_key());
    }

    // =====================================================
    // build_findref_query テスト
    // =====================================================

    #[test]
    fn test_build_findref_query_with_limit_offset() {
        let param = GetParam {
            limit: Some(100),
            offset: Some(50),
            ..Default::default()
        };
        let builder = FindRefBuilder::default().sota();
        let query = build_findref_query(param, builder).unwrap();

        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(50));
    }

    #[test]
    fn test_build_findref_query_with_codes() {
        let param = GetParam {
            sota_code: Some("JA/TK-001".to_string()),
            pota_code: Some("JA-0001".to_string()),
            ..Default::default()
        };
        let builder = FindRefBuilder::default().sota();
        let query = build_findref_query(param, builder).unwrap();

        assert_eq!(query.sota_code, Some("JA/TK-001".to_string()));
        assert_eq!(query.pota_code, Some("JA-0001".to_string()));
    }

    #[test]
    fn test_build_findref_query_with_bbox() {
        let param = GetParam {
            min_lon: Some(139.0),
            min_lat: Some(35.0),
            max_lon: Some(140.0),
            max_lat: Some(36.0),
            ..Default::default()
        };
        let builder = FindRefBuilder::default().sota();
        let query = build_findref_query(param, builder).unwrap();

        assert!(query.bbox.is_some());
    }

    #[test]
    fn test_build_findref_query_with_center() {
        let param = GetParam {
            lon: Some(139.7),
            lat: Some(35.6),
            dist: Some(10.0),
            ..Default::default()
        };
        let builder = FindRefBuilder::default().pota();
        let query = build_findref_query(param, builder).unwrap();

        assert!(query.center.is_some());
    }

    #[test]
    fn test_build_findref_query_bbox_takes_priority_over_center() {
        // bbox と center の両方が指定された場合、bboxが優先される
        let param = GetParam {
            min_lon: Some(139.0),
            min_lat: Some(35.0),
            max_lon: Some(140.0),
            max_lat: Some(36.0),
            lon: Some(139.7),
            lat: Some(35.6),
            dist: Some(10.0),
            ..Default::default()
        };
        let builder = FindRefBuilder::default().sota();
        let query = build_findref_query(param, builder).unwrap();

        // bboxが設定されていることを確認（bbox優先）
        assert!(query.bbox.is_some());
        // centerは設定されない
        assert!(query.center.is_none());
    }

    #[test]
    fn test_build_findref_query_partial_bbox_ignored() {
        // bbox パラメータが部分的な場合は無視される
        let param = GetParam {
            min_lon: Some(139.0),
            min_lat: Some(35.0),
            // max_lon, max_lat がない
            ..Default::default()
        };
        let builder = FindRefBuilder::default().sota();
        let query = build_findref_query(param, builder).unwrap();

        assert!(query.bbox.is_none());
        assert!(query.center.is_none());
    }

    #[test]
    fn test_build_findref_query_with_elevation() {
        let param = GetParam {
            min_elev: Some(1000),
            ..Default::default()
        };
        let builder = FindRefBuilder::default().sota();
        let query = build_findref_query(param, builder).unwrap();

        assert_eq!(query.min_elev, Some(1000));
    }

    #[test]
    fn test_build_findref_query_with_name() {
        let param = GetParam {
            name: Some("富士山".to_string()),
            ..Default::default()
        };
        let builder = FindRefBuilder::default().sota();
        let query = build_findref_query(param, builder).unwrap();

        assert_eq!(query.name, Some("富士山".to_string()));
    }
}
