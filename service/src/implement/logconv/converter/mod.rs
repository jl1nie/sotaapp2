//! 変換: 読み取った交信(`QsoRecord`)から、SOTA の CSV・ADIF の出力ファイルを作る
//!
//! - `sota`: SOTA アクティベーター(CSV・S2S の CSV・ADIF)、SOTA チェイサー(CSV)
//! - `adif`: 自局の参照ごとの ADIF(POTA・WWFF・SOTA)と、確認画面に出す表
//! - `refs`: 備考・QTH の列から、自局・相手の参照を取る
//! - `files`: 出力ファイルの集まり、CSV の書式、ZIP
//!
//! 旧版(HamLogConv の Python)と同じ出力にする。違う所は、各所のコメントに理由を書いてある。

mod adif;
mod files;
mod refs;
mod sota;

pub use adif::convert_to_adif;
pub use files::create_zip;
pub use sota::{convert_to_sota_activator, convert_to_sota_chaser};

#[cfg(test)]
mod tests;
