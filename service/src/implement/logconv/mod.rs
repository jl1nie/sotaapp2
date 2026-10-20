//! ログの変換: HAMLOG の CSV・HamLog iOS の CSV・ADIF を読んで、SOTA の CSV と ADIF にする
//!
//! 入力(読み取り): `hamlog`(HAMLOG・HamLog iOS)、`adif`(ADIF)→ `record`(交信 `QsoRecord`)
//! 出力(変換): `converter`(SOTA アクティベーター・チェイサー、POTA・WWFF・SOTA の ADIF)
//!
//! 部品: `bands`(周波数→バンド)、`modes`(モードの名前)、`callsign`(コールサインの分解)、
//! `reference`(備考・QTH の参照番号)、`options`(変換のオプションと結果)
//!
//! 旧版(HamLogConv の Python)と同じ出力にする(`test_data/logconv` に、旧版の出力を正解として置いてある)。

pub mod adif;
pub mod bands;
pub mod callsign;
pub mod converter;
pub mod hamlog;
pub mod modes;
pub mod options;
pub mod record;
pub mod reference;

pub use adif::*;
pub use bands::*;
pub use callsign::{normalize_callsign, normalize_width, split_callsign};
pub use converter::*;
pub use hamlog::*;
pub use modes::*;
pub use options::*;
pub use record::*;
pub use reference::*;
