//! 読み取った交信(`QsoRecord`)と、読み取れなかった理由(`RecordError`)

use std::fmt;

/// 交信を読み取れなかった理由
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    /// HAMLOG の CSV の形式でない(列が足りないなど)
    NotHamlogCsv,
    /// HamLog iOS の CSV の列が足りない
    TooFewColumns(usize),
    /// 日付の書式が違う(`YY/MM/DD` でも `YYYY/MM/DD` でもない)
    BadDate(String),
    /// 時刻の書式が違う(`HH:MM` に、`U` か `J` が付く形でない)
    BadTime(String),
    /// 書式は合っているが、存在しない日時(`99:99`、13月など)
    NonexistentDateTime { date: String, time: String },
    /// どのバンドにもない周波数
    BadFrequency(String),
    /// ADIF の記録が読めない
    BadAdif(String),
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHamlogCsv => write!(f, "HAMLOG CSV形式ではありません"),
            Self::TooFewColumns(n) => write!(f, "列が足りません({n}列)"),
            Self::BadDate(text) => write!(f, "日付フォーマット不正:{text}"),
            Self::BadTime(text) => write!(f, "時刻フォーマット不正:{text}"),
            Self::NonexistentDateTime { date, time } => write!(f, "日時不正:{date} {time}"),
            Self::BadFrequency(reason) => write!(f, "周波数不正:{reason}"),
            Self::BadAdif(reason) => write!(f, "ADIF形式が不正です:{reason}"),
        }
    }
}

/// 読み取った交信(HAMLOG CSV・HamLog iOS・ADIF のどれからでも、同じ形)
///
/// 日付と時刻は UTC。読み取れない項目があった交信は、`errors` にその理由が入る。
#[derive(Debug, Clone, Default)]
pub struct QsoRecord {
    /// 入力ファイルでの行番号(1から。分からなければ 0)。誤りの知らせに使う
    pub source_line: usize,
    /// 読み取れなかった理由(なければ、読み取れた交信)
    pub errors: Vec<RecordError>,

    /// 相手のコールサイン
    pub callsign: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub rst_sent: String,
    pub rst_rcvd: String,
    /// バンドの、SOTA での名前(`7MHz`、`VLF`、`433MHz` など)
    pub band_sota: String,
    /// バンドの波長での名前(`40m`)。ADIF の BAND
    pub band_wlen: String,
    /// ADIF のモード(`CW`、`SSB`、`MFSK` など)
    pub mode: String,
    /// ADIF のサブモード(`FT4` など。なければ空)
    pub sub_mode: String,
    /// SOTA のモード(`CW`、`SSB`、`DATA` など)
    pub mode_sota: String,
    /// QTH の列。相手の参照を置くことがある
    pub qth: String,
    /// 備考1の列。自局の参照を置くことが多い
    pub remarks1: String,
    /// 備考2の列
    pub remarks2: String,
}

impl QsoRecord {
    /// 読み取れなかった行の記録(誤りの行として、変換の結果に残す)
    pub fn unreadable(source_line: usize, error: RecordError) -> Self {
        Self {
            source_line,
            errors: vec![error],
            ..Default::default()
        }
    }

    /// 読み取れなかった項目がある
    pub fn has_error(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 読み取れなかった理由(複数あれば、`, ` でつなぐ)
    pub fn error_message(&self) -> String {
        self.errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}
