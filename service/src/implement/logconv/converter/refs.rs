//! 備考・QTH の列から、自局・相手の参照を取る。誤りの知らせの行番号

use super::super::types::{get_ref, ConversionOptions, HisQth, MyQth, QsoRecord, RefInfo};

/// 交信の日付(`YYYYMMDD`。UTC)
pub fn date_key(qso: &QsoRecord) -> String {
    format!("{:04}{:02}{:02}", qso.year, qso.month, qso.day)
}

/// 誤りの知らせに出す行番号。入力ファイルの行番号が分かればそれ、なければ交信の順番
pub fn line_of(qso: &QsoRecord, index: usize) -> usize {
    if qso.source_line > 0 {
        qso.source_line
    } else {
        index + 1
    }
}

/// 自局の参照(SOTA アクティベーター)。`Form` は、画面で入力した山
pub fn my_ref_for_sota(qso: &QsoRecord, options: &ConversionOptions) -> RefInfo {
    match options.my_qth {
        MyQth::Rmks1 => get_ref(&qso.remarks1),
        MyQth::Rmks2 => get_ref(&qso.remarks2),
        MyQth::Form => RefInfo {
            sota: options.summit.clone(),
            ..Default::default()
        },
    }
}

/// 自局の参照(ADIF)。`Form` は、画面で入力した参照。`JAFF-0005 JP-0014` のように、複数を書ける
pub fn my_ref_for_adif(qso: &QsoRecord, options: &ConversionOptions) -> RefInfo {
    match options.my_qth {
        MyQth::Rmks1 => get_ref(&qso.remarks1),
        MyQth::Rmks2 => get_ref(&qso.remarks2),
        MyQth::Form => get_ref(&options.park.join(" ")),
    }
}

/// 相手の参照
pub fn his_ref(qso: &QsoRecord, his_qth: HisQth) -> RefInfo {
    match his_qth {
        HisQth::Rmks1 => get_ref(&qso.remarks1),
        HisQth::Rmks2 => get_ref(&qso.remarks2),
        HisQth::Qth => get_ref(&qso.qth),
        HisQth::None => RefInfo::default(),
    }
}
