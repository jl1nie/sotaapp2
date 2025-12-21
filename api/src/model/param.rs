use axum::{
    extract::{FromRequestParts, Query},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::str::FromStr;
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

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
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
