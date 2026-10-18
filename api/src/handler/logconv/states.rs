//! 運用した公園(POTA)の所在地(ADIF の MY_STATE)

use common::error::AppError;
use domain::model::event::FindRefBuilder;
use service::implement::logconv::{get_ref, ConversionOptions, MyQth, QsoRecord};
use service::services::AdminService;
use std::collections::HashMap;

/// 運用した公園の所在地を、サーバーの公園データから引く。例: `JA-0001` → `TK`
///
/// 旧版と同じく、所在地が1つに決まる公園だけ(複数の都道府県にまたがる公園は、入れない)。
pub async fn park_states(
    admin: &dyn AdminService,
    records: &[QsoRecord],
    options: &ConversionOptions,
) -> HashMap<String, String> {
    let mut states = HashMap::new();
    for code in operated_parks(records, options) {
        let query = FindRefBuilder::default()
            .pota()
            .pota_code(code.clone())
            .build();
        let park = match admin.show_pota_reference(query).await {
            Ok(park) => park,
            Err(AppError::RowNotFound { .. }) => continue,
            Err(e) => {
                tracing::warn!("park location lookup failed for {code}: {e}");
                continue;
            }
        };
        if let Some(state) = single_state(&park.park_locid) {
            states.insert(code, state);
        }
    }
    states
}

/// ログで運用した公園の番号(重複なし。ログに現れた順)
fn operated_parks(records: &[QsoRecord], options: &ConversionOptions) -> Vec<String> {
    let mut codes: Vec<String> = Vec::new();
    for qso in records {
        let mine = match options.my_qth {
            MyQth::Rmks1 => get_ref(&qso.remarks1).pota,
            MyQth::Rmks2 => get_ref(&qso.remarks2).pota,
            MyQth::Form => get_ref(&options.park.join(" ")).pota,
        };
        for code in mine {
            if !codes.contains(&code) {
                codes.push(code);
            }
        }
    }
    codes
}

/// `JP-TK` → `TK`。カンマで区切った複数の所在地(`JP-TK,JP-SI`)は None
pub fn single_state(locid: &str) -> Option<String> {
    let mut parts = locid.split(',').map(str::trim).filter(|p| !p.is_empty());
    let first = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some(first.trim_start_matches("JP-").to_string())
}
