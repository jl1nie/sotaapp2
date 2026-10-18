//! 出力ファイルの集まり、CSV の書式、ZIP

use super::super::types::{adif_field, AdifField};
use std::collections::HashMap;
use std::io::Write;
use zip::write::SimpleFileOptions;

/// 出力ファイルの集まり(ファイル名 → 内容)。同じ名前には、追記する
#[derive(Debug, Default)]
pub struct FileSet(HashMap<String, String>);

impl FileSet {
    /// ファイルに追記する(なければ作る)
    pub fn append(&mut self, name: impl Into<String>, text: &str) {
        self.0.entry(name.into()).or_default().push_str(text);
    }

    /// ファイルに追記する。まだなければ、`header` から始める
    pub fn append_with_header(&mut self, name: impl Into<String>, header: &str, text: &str) {
        self.0
            .entry(name.into())
            .or_insert_with(|| header.to_string())
            .push_str(text);
    }

    /// ファイルを、内容ごと置く
    pub fn insert(&mut self, name: impl Into<String>, body: String) {
        self.0.insert(name.into(), body);
    }

    pub fn into_files(self) -> HashMap<String, String> {
        self.0
    }
}

/// ADIF のファイルの先頭(`<EOH>` まで)。`adif_version` は、`ADIF_VER` に入れる版
pub fn adif_header(adif_version: &str) -> String {
    format!(
        "ADIF Export from HAMLOG by JL1NIE\n{}\n{}\n<EOH>\n",
        adif_field(AdifField::ProgramId, "FCTH"),
        adif_field(AdifField::AdifVer, adif_version)
    )
}

/// CSV の1行。コンマ・引用符・改行を含む項目だけを `"` で囲む(旧版の csv.writer の QUOTE_MINIMAL と同じ)
pub fn csv_line(row: &[String]) -> String {
    row.iter()
        .map(|f| {
            if f.contains([',', '"', '\n', '\r']) {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                f.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Create a ZIP file from a hashmap of filename -> content
pub fn create_zip(files: &HashMap<String, String>) -> Result<Vec<u8>, std::io::Error> {
    let mut buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, content) in files {
            zip.start_file(name, options)?;
            zip.write_all(content.as_bytes())?;
        }
        zip.finish()?;
    }
    Ok(buffer)
}
