//! 新しいテスト(options・errors・encoding)が共有する道具
//!
//! 親の `tests.rs` の `multipart_of_bytes` は `file` と `options` を必ず付けるので、
//! 欠けた multipart や、順番・名前の違う multipart を作れるように、パーツを指定して組み立てる。

use super::*;
use axum::extract::FromRequest;
use std::collections::BTreeMap;

/// multipart の1パート。`filename` があればファイルとして送る
pub struct Part<'a> {
    pub name: &'a str,
    pub filename: Option<&'a str>,
    pub body: &'a [u8],
}

impl<'a> Part<'a> {
    pub fn file(body: &'a [u8]) -> Self {
        Self {
            name: "file",
            filename: Some("log.csv"),
            body,
        }
    }

    pub fn field(name: &'a str, body: &'a str) -> Self {
        Self {
            name,
            filename: None,
            body: body.as_bytes(),
        }
    }
}

/// パーツから multipart のリクエストを作る(ハンドラーに渡す `Multipart`)
pub async fn multipart_parts(parts: &[Part<'_>]) -> Multipart {
    let boundary = "XBOUNDARYX";
    let mut body: Vec<u8> = Vec::new();
    for p in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match p.filename {
            Some(f) => body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{}\"; filename=\"{f}\"\r\n\r\n",
                    p.name
                )
                .as_bytes(),
            ),
            None => body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                    p.name
                )
                .as_bytes(),
            ),
        }
        body.extend_from_slice(p.body);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    let request = axum::http::Request::builder()
        .method("POST")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(axum::body::Body::from(body))
        .unwrap();
    Multipart::from_request(request, &()).await.unwrap()
}

/// ハンドラーの応答(ステータス・種類・本文)
pub struct Reply {
    pub status: StatusCode,
    pub is_zip: bool,
    pub body: Vec<u8>,
}

impl Reply {
    pub async fn of(res: Response) -> Self {
        let status = res.status();
        let is_zip = res
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|t| t == "application/zip");
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        Self {
            status,
            is_zip,
            body,
        }
    }

    /// ZIP の中身(ファイル名 → 本文。CRLF は LF にそろえる)
    pub fn zip(&self) -> BTreeMap<String, String> {
        assert!(self.is_zip, "ZIP ではない応答: {}", self.text());
        unzip(&self.body)
    }

    /// JSON の応答
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("JSON ではない応答({e}): {}", self.text()))
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// 誤りの行の番号(`error_log` の `N行:…` の N)
    pub fn error_lines(&self) -> Vec<u64> {
        self.json()["error_log"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|l| l.as_str()?.split('行').next()?.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// `/logconv/hamlog` に送る
pub async fn hamlog(file: &[u8], options: &str) -> Reply {
    Reply::of(hamlog_convert(None, multipart_of_bytes(file, options).await).await).await
}

/// `/logconv/pota`(確認)に送る
pub async fn confirm(file: &[u8], options: &str) -> Reply {
    Reply::of(pota_check(None, multipart_of_bytes(file, options).await).await).await
}

/// どの応答も満たすべき決まり: 500 でなく、失敗(4xx)なら利用者に読めるメッセージ(JSON の `status: NG` と `error`)がある
pub fn assert_well_formed(what: &str, r: &Reply) {
    assert!(
        !r.status.is_server_error(),
        "{what}: サーバーの失敗 {}: {}",
        r.status,
        r.text()
    );
    if r.status.is_client_error() {
        let j: serde_json::Value = serde_json::from_slice(&r.body)
            .unwrap_or_else(|_| panic!("{what}: 4xx の本文が JSON でない: {}", r.text()));
        assert_eq!(j["status"], "NG", "{what}");
        assert!(
            j["error"].as_str().is_some_and(|e| !e.is_empty()),
            "{what}: error のメッセージがない: {j}"
        );
    }
}

/// HAMLOG CSV の1行(列: コール,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,)
pub fn row(
    call: &str,
    date: &str,
    time: &str,
    name: &str,
    qth: &str,
    r1: &str,
    r2: &str,
) -> String {
    format!("{call},{date},{time},59,59,7.030,CW,,,N,{name},{qth},{r1},{r2},")
}

/// SOTA アクティベーターで自局=備考1、相手=QTH の指定
pub const SOTA_A: &str =
    r#"{"format":"sota_a","myQTH":"rmks1","QTH":"qth","SOTAActivator":"JA1ZZZ/1"}"#;
/// SOTA チェイサー
pub const SOTA_C: &str = r#"{"format":"sota_c","QTH":"qth","callsign":"JA1ZZZ"}"#;
/// POTA(自局=備考1、相手=QTH)
pub const POTA: &str = r#"{"format":"pota","myQTH":"rmks1","QTH":"qth","POTAActivator":"JA1ZZZ/1","POTAOperator":"JA1ZZZ"}"#;

/// パニックを捕まえて送る(None ならパニックした)。パニックはどの入力でも不具合
pub async fn try_hamlog(file: &[u8], options: &str) -> Option<Reply> {
    use futures_util::FutureExt;
    let fut = async { hamlog(file, options).await };
    std::panic::AssertUnwindSafe(fut).catch_unwind().await.ok()
}

pub async fn try_confirm(file: &[u8], options: &str) -> Option<Reply> {
    use futures_util::FutureExt;
    let fut = async { confirm(file, options).await };
    std::panic::AssertUnwindSafe(fut).catch_unwind().await.ok()
}
