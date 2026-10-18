//! 入力の受け取り: アップロード(multipart)、文字コード、ログの読み取り

use axum::extract::Multipart;
use serde::de::DeserializeOwned;
use service::implement::logconv::{
    decode_adif, decode_auto, parse_adif, ConversionOptions, HisQth, QsoRecord,
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
        while let Ok(Some(field)) = multipart.next_field().await {
            match field.name() {
                Some("file") => {
                    if let Ok(bytes) = field.bytes().await {
                        file = Some(bytes.to_vec());
                    }
                }
                Some("options") | Some("arg") => {
                    if let Ok(text) = field.text().await {
                        options_json = Some(text);
                    }
                }
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
        let (records, is_adif) = read_records(&decode_text(&self.file));
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

/// 入力ファイルの文字コードを判定して読む。UTF-8 として正しければ UTF-8、そうでなければ Shift-JIS(cp932)
///
/// 日本語版の HAMLOG が出す CSV は Shift-JIS(旧版は常に cp932 で読んでいた)。
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}

/// 入力のファイルを読む。1行目に `ADIF` か `<EOH>` があれば ADIF、なければ HAMLOG CSV(HamLog iOS を含む)
pub fn read_records(content: &str) -> (Vec<QsoRecord>, bool) {
    let first_line = content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .to_uppercase();
    let is_adif = first_line.contains("ADIF") || first_line.contains("<EOH>");
    let records = if is_adif {
        read_adif(content)
    } else {
        read_hamlog_csv(content)
    };
    (records, is_adif)
}

fn read_adif(content: &str) -> Vec<QsoRecord> {
    parse_adif(content)
        .iter()
        .map(|fields| {
            // ADIFレコードを文字列に再構築
            let record: String = fields
                .iter()
                .map(|(k, v)| format!("<{}:{}>{}", k, v.len(), v))
                .collect::<String>()
                + "<EOR>";
            decode_adif(&record).unwrap_or_else(|e| QsoRecord::unreadable(0, e))
        })
        .collect()
}

fn read_hamlog_csv(content: &str) -> Vec<QsoRecord> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut records = Vec::new();
    let mut header_seen = false;
    for row in reader.records().flatten() {
        let cols: Vec<String> = row.iter().map(str::to_string).collect();
        if cols.is_empty() {
            continue;
        }
        let line = row.position().map_or(0, |p| p.line() as usize);
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
