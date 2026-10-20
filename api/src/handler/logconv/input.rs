//! 入力の受け取り: アップロード(multipart)、文字コード、ログの読み取り

use axum::extract::multipart::MultipartError;
use axum::extract::Multipart;
use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use service::implement::logconv::{
    decode_adif_fields, decode_auto, parse_adif_bytes, ConversionOptions, HisQth, QsoRecord,
};
use service::services::AdminService;

use super::options::ConversionOptionsRequest;
use super::reply::Failure;
use super::states::park_states;

/// アップロードされたもの(ログのファイルと、`options` の JSON)
pub struct Upload {
    file: Vec<u8>,
    options_json: Option<String>,
}

/// 読み取ったログと、それに対する変換のオプション
pub struct Loaded {
    pub records: Vec<QsoRecord>,
    pub options: ConversionOptions,
}

impl Upload {
    /// multipart の `file` と、`options`(`arg` でもよい)を取り出す。`file` がなければ 400
    pub async fn read(mut multipart: Multipart) -> Result<Self, Failure> {
        let mut file = None;
        let mut options_json = None;
        loop {
            let field = match multipart.next_field().await {
                Ok(Some(field)) => field,
                Ok(None) => break,
                Err(e) => return Err(read_failure(&e)),
            };
            match field.name() {
                Some("file") => match field.bytes().await {
                    Ok(bytes) => file = Some(bytes.to_vec()),
                    Err(e) => return Err(read_failure(&e)),
                },
                Some("options") | Some("arg") => match field.text().await {
                    Ok(text) => options_json = Some(text),
                    Err(e) => return Err(read_failure(&e)),
                },
                _ => {}
            }
        }
        match file {
            Some(file) => Ok(Self { file, options_json }),
            None => Err(Failure::bad_request("Missing 'file' parameter")),
        }
    }

    /// `options` を読む。なかったり、読めなかったりしたら 400
    pub fn options_strict<T: DeserializeOwned>(&self) -> Result<T, Failure> {
        match &self.options_json {
            Some(json) => serde_json::from_str(json)
                .map_err(|e| Failure::bad_request(format!("Invalid options JSON: {e}"))),
            None => Err(Failure::bad_request("Missing 'options' or 'arg' parameter")),
        }
    }

    /// `options` を読む。なかったり、読めなかったりしたら、既定の指定
    pub fn options_or_default<T: DeserializeOwned + Default>(&self) -> T {
        self.options_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default()
    }

    /// ログを読み、オプションを決める。読めた交信が1つもなければ 400
    pub async fn load(
        &self,
        request: ConversionOptionsRequest,
        admin: Option<&dyn AdminService>,
    ) -> Result<Loaded, Failure> {
        let (records, is_adif) = read_log(&self.file);
        if records.is_empty() {
            return Err(Failure::bad_request("No valid records found"));
        }

        let mut options: ConversionOptions = request.into();
        if is_adif {
            // ADIF の入力には、HAMLOG の QTH の列がない。相手の参照は、ADIF のコメントなどから取る
            options.his_qth = HisQth::Qth;
        }
        if let Some(admin) = admin {
            options.park_states = park_states(admin, &records, &options).await;
        }
        Ok(Loaded { records, options })
    }
}

/// multipart の読み込みの失敗。上限(64MB)を超えたときは 413、それ以外は、ファイルが受け取れなかったとして 400
fn read_failure(e: &MultipartError) -> Failure {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        Failure::payload_too_large("ファイルが大きすぎます(上限 64MB)")
    } else {
        Failure::bad_request("Missing 'file' parameter")
    }
}

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// 末尾の 0x1A(古い DOS のファイルの終わりの印)と、それに続く空白を除く
fn strip_sub_eof(bytes: &[u8]) -> &[u8] {
    // 末尾の、空白と 0x1A だけが並ぶ部分に 0x1A があれば、最初の 0x1A から後ろを捨てる(その前の改行は残す)
    let tail = bytes
        .iter()
        .rposition(|&b| b != 0x1A && !b.is_ascii_whitespace())
        .map_or(0, |p| p + 1);
    match bytes[tail..].iter().position(|&b| b == 0x1A) {
        Some(p) => &bytes[..tail + p],
        None => bytes,
    }
}

/// 入力ファイルの文字コードを判定して読む。UTF-8 として正しければ UTF-8、そうでなければ Shift-JIS(cp932)
///
/// 日本語版の HAMLOG が出す CSV は Shift-JIS(旧版は常に cp932 で読んでいた)。
/// 末尾の 0x1A は無視する。
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = strip_sub_eof(bytes);
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}

/// 入力のファイルを読む。1行目に `ADIF` か `<EOH>` があれば ADIF、なければ HAMLOG CSV(HamLog iOS を含む)
///
/// ADIF の項目の長さ(`<COMMENT:4>`)は、元のファイルの文字コードでのバイト数なので、
/// バイト列のまま読んで、値ごとに文字列にする。
pub fn read_log(bytes: &[u8]) -> (Vec<QsoRecord>, bool) {
    let bytes = strip_sub_eof(bytes);
    let content = decode_text(bytes);
    let first_line = content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .to_uppercase();
    let is_adif = first_line.contains("ADIF") || first_line.contains("<EOH>");
    let records = if is_adif {
        read_adif(bytes, &content)
    } else {
        read_hamlog_csv(&content)
    };
    (records, is_adif)
}

fn read_adif(bytes: &[u8], content: &str) -> Vec<QsoRecord> {
    let plain = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    let utf16 = bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]);
    let parsed = if std::str::from_utf8(plain).is_ok() {
        parse_adif_bytes(plain, &|v| String::from_utf8_lossy(v).into_owned())
    } else if utf16 {
        // UTF-16 は、文字列にしたものを UTF-8 として読む(長さは UTF-8 のバイト数)
        parse_adif_bytes(content.as_bytes(), &|v| {
            String::from_utf8_lossy(v).into_owned()
        })
    } else {
        parse_adif_bytes(plain, &|v| {
            encoding_rs::SHIFT_JIS
                .decode_without_bom_handling(v)
                .0
                .into_owned()
        })
    };
    parsed
        .iter()
        .map(|fields| decode_adif_fields(fields).unwrap_or_else(|e| QsoRecord::unreadable(0, e)))
        .collect()
}

fn read_hamlog_csv(content: &str) -> Vec<QsoRecord> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());

    let line_starts = line_starts(content);
    let mut records = Vec::new();
    let mut header_seen = false;
    for row in reader.records().flatten() {
        let cols: Vec<String> = row.iter().map(str::to_string).collect();
        if cols.is_empty() {
            continue;
        }
        let line = row
            .position()
            .map_or(0, |p| line_at(content, &line_starts, p.byte() as usize));
        match decode_auto(&cols, header_seen) {
            Ok(Some(mut qso)) => {
                qso.source_line = line;
                records.push(qso);
            }
            // 見出しの行は、読み飛ばす
            Ok(None) => {}
            // 列が足りないなど、読み取れない行は、誤りの行として残す(黙って捨てない)
            Err(e) => records.push(QsoRecord::unreadable(line, e)),
        }
        header_seen = true;
    }
    records
}

/// 各行の開始位置。改行は CRLF・LF・CR のどれも行の区切りとして数える(csv の読み方と同じ)
pub(super) fn line_starts(content: &str) -> Vec<usize> {
    let bytes = content.as_bytes();
    let mut starts = vec![0];
    for (i, &b) in bytes.iter().enumerate() {
        let ends_line = b == b'\n' || (b == b'\r' && bytes.get(i + 1) != Some(&b'\n'));
        if ends_line {
            starts.push(i + 1);
        }
    }
    starts
}

/// 入力ファイルの、その位置から始まる行の番号(1から)。行の開始位置を先に作ってあるので、二分探索で引く
///
/// csv の `Position::line` は、改行が CRLF(Windows の HAMLOG の CSV)だと、2行目から1つ小さくなる。
/// 記録の位置が、前の行の改行の途中を指すことがあるので、改行を読み飛ばしてから数える。
fn line_at(content: &str, line_starts: &[usize], byte: usize) -> usize {
    let bytes = content.as_bytes();
    let start = byte.min(bytes.len());
    let skip = bytes[start..]
        .iter()
        .take_while(|&&b| b == b'\r' || b == b'\n')
        .count();
    line_starts.partition_point(|&s| s <= start + skip)
}
