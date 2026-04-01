/// APRS接続診断テスト
///
/// 目的: buddy filterが効いていないのか、上流からパケットが来ていないのかを切り分ける
///
/// 実行方法:
///   cargo nextest run --package adapter aprs_diagnosis --test-threads=1 --no-capture
///
/// 環境変数 (.envが必要):
///   APRSHOST, APRSUSER, APRSPASSWORD
use aprs_message::{AprsData, AprsIS};
use std::env;
use tokio::time::{timeout, Duration};

fn load_env() {
    // .envファイルを読み込む（存在すれば）
    if let Ok(contents) = std::fs::read_to_string(".env") {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                let val = val.trim_matches('"');
                unsafe {
                    std::env::set_var(key.trim(), val);
                }
            }
        }
    }
}

/// フェーズ1: フィルターなしで接続し、上流からパケットが届くか確認
///
/// フィルターなしの場合、APRS-ISサーバーは接続直後から全パケットを流してくる。
/// 30秒以内に何かパケットが来ればAPRS-IS接続自体は正常。
#[tokio::test]
#[ignore] // 実ネットワーク接続が必要。 --ignored フラグで実行
async fn phase1_no_filter_receives_any_packet() {
    load_env();
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();

    let host = env::var("APRSHOST").unwrap_or_else(|_| "rotate.aprs2.net:14580".to_string());
    let user = env::var("APRSUSER").expect("APRSUSER not set");
    let password = env::var("APRSPASSWORD").expect("APRSPASSWORD not set");

    tracing::info!("=== Phase 1: フィルターなし接続テスト ===");
    tracing::info!("接続先: {}", host);
    tracing::info!("ユーザー: {}", user);

    let server = AprsIS::connect(&host, &user, &password)
        .await
        .expect("APRS-IS接続失敗");

    tracing::info!("接続成功。30秒間パケット受信を待機...");

    let mut count = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, server.read_packet()).await {
            Ok(Ok(packet)) => {
                count += 1;
                match &packet {
                    AprsData::AprsPosition {
                        callsign,
                        latitude,
                        longitude,
                    } => {
                        tracing::info!(
                            "[{}] Position: {}/{} ({:.4}, {:.4})",
                            count,
                            callsign.callsign,
                            callsign.ssid.map(|s| s.to_string()).unwrap_or_default(),
                            latitude,
                            longitude
                        );
                    }
                    AprsData::AprsMessage {
                        callsign,
                        addressee,
                        message,
                    } => {
                        tracing::info!(
                            "[{}] Message: {} -> {}: {}",
                            count,
                            callsign.callsign,
                            addressee,
                            message
                        );
                    }
                }
                if count >= 5 {
                    tracing::info!("5パケット受信完了。上流は正常にパケットを送信しています。");
                    break;
                }
            }
            Ok(Err(e)) => {
                tracing::error!("パケット受信エラー: {:?}", e);
                break;
            }
            Err(_) => {
                tracing::warn!("タイムアウト: 30秒以内にパケットが届きませんでした");
                break;
            }
        }
    }

    tracing::info!("=== Phase 1 結果: {}パケット受信 ===", count);
    assert!(
        count > 0,
        "フィルターなしでもパケットが届きません。APRS-IS接続またはサーバー設定に問題があります"
    );
}

/// フェーズ2: 東京エリアのレンジフィルターでパケット受信確認
///
/// 地理的フィルターを使って日本のパケットが届くか確認。
/// これが届けば、フィルター構文は問題なし。
#[tokio::test]
#[ignore]
async fn phase2_range_filter_japan() {
    load_env();
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();

    let host = env::var("APRSHOST").unwrap_or_else(|_| "rotate.aprs2.net:14580".to_string());
    let user = env::var("APRSUSER").expect("APRSUSER not set");
    let password = env::var("APRSPASSWORD").expect("APRSPASSWORD not set");

    tracing::info!("=== Phase 2: 日本エリアレンジフィルターテスト ===");

    let server = AprsIS::connect(&host, &user, &password)
        .await
        .expect("APRS-IS接続失敗");

    // 東京中心 500km圏内
    let filter = "r/35.68/139.77/500".to_string();
    tracing::info!("フィルター設定: {}", filter);
    server.set_filter(filter).await.expect("フィルター設定失敗");

    tracing::info!("60秒間パケット受信を待機...");

    let mut count = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, server.read_packet()).await {
            Ok(Ok(packet)) => {
                count += 1;
                match &packet {
                    AprsData::AprsPosition {
                        callsign,
                        latitude,
                        longitude,
                    } => {
                        tracing::info!(
                            "[{}] Position: {}-{}: ({:.4}, {:.4})",
                            count,
                            callsign.callsign,
                            callsign.ssid.map(|s| s.to_string()).unwrap_or_default(),
                            latitude,
                            longitude
                        );
                    }
                    AprsData::AprsMessage {
                        callsign,
                        addressee,
                        message,
                    } => {
                        tracing::info!(
                            "[{}] Message: {} -> {}: {}",
                            count,
                            callsign.callsign,
                            addressee,
                            message
                        );
                    }
                }
                if count >= 5 {
                    tracing::info!("5パケット受信完了。日本エリアのフィルターは正常動作。");
                    break;
                }
            }
            Ok(Err(e)) => {
                tracing::error!("エラー: {:?}", e);
                break;
            }
            Err(_) => {
                tracing::warn!("タイムアウト: 60秒以内にパケットが届きませんでした");
                break;
            }
        }
    }

    tracing::info!("=== Phase 2 結果: {}パケット受信 ===", count);
    assert!(count > 0, "日本エリアフィルターでパケットが届きません");
}

/// フェーズ3: レンジフィルターで捕捉した局をそのままbuddyに設定して動作確認
///
/// Phase 2で実際に動いている局をbuddyに設定することで、
/// 「局が出ていないせいか」vs「buddy filter自体が効かないのか」を切り分ける。
#[tokio::test]
#[ignore]
async fn phase3_buddy_filter_from_live_stations() {
    load_env();
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init()
        .ok();

    let host = env::var("APRSHOST").unwrap_or_else(|_| "rotate.aprs2.net:14580".to_string());
    let user = env::var("APRSUSER").expect("APRSUSER not set");
    let password = env::var("APRSPASSWORD").expect("APRSPASSWORD not set");

    tracing::info!("=== Phase 3: live stationsをbuddyに設定してテスト ===");

    // 同一接続でレンジフィルター→buddy切り替えテスト
    // これにより「局が出ていない」問題を完全に排除する
    tracing::info!("[Step 1] 同一接続でレンジフィルターで局を収集中 (15秒)...");
    let server = AprsIS::connect(&host, &user, &password)
        .await
        .expect("APRS-IS接続失敗");
    server
        .set_filter("r/35.68/139.77/500".to_string())
        .await
        .expect("レンジフィルター設定失敗");

    // 短いビーコン間隔の局を見つけるため、30秒間パケットを収集し
    // 複数回出現した局（= 頻繁にビーコン）を優先して選ぶ
    let mut callsign_count: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    let deadline1 = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut range_count = 0;
    loop {
        let remaining = deadline1.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, server.read_packet()).await {
            Ok(Ok(AprsData::AprsPosition { callsign, .. })) => {
                range_count += 1;
                let cnt = callsign_count.entry(callsign.callsign.clone()).or_insert(0);
                *cnt += 1;
                if *cnt == 1 {
                    tracing::info!("[range {}] 初出現: {}", range_count, callsign.callsign);
                } else {
                    tracing::info!(
                        "[range {}] 再出現({}回目): {}",
                        range_count,
                        cnt,
                        callsign.callsign
                    );
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    // 複数回出現した局を優先、足りなければ1回出現の局で補完
    let mut frequent: Vec<String> = callsign_count
        .iter()
        .filter(|(_, &v)| v >= 2)
        .map(|(k, _)| k.clone())
        .collect();
    let mut once: Vec<String> = callsign_count
        .iter()
        .filter(|(_, &v)| v == 1)
        .map(|(k, _)| k.clone())
        .collect();
    frequent.sort();
    once.sort();
    let live_callsigns: Vec<String> = {
        let mut v = frequent;
        v.extend(once.into_iter().take(5usize.saturating_sub(v.len())));
        v.truncate(5);
        v
    };
    tracing::info!(
        "選択局 ({} 局, 複数回出現優先): {:?}",
        live_callsigns.len(),
        live_callsigns
    );
    tracing::info!("全出現回数: {:?}", callsign_count);
    assert!(
        !live_callsigns.is_empty(),
        "レンジフィルターで局を収集できませんでした"
    );

    // [Step 2] 同一接続でbuddyフィルターに切り替え
    // 仕様: b/CALL はSSIDなしのみマッチ。b/CALL* で全SSIDマッチ
    // 収集した局は全てSSID付き（-2, -9 等）なので * ワイルドカードが必要
    let buddies_with_wildcard: Vec<String> =
        live_callsigns.iter().map(|c| format!("{}*", c)).collect();
    tracing::info!("[Step 2] 同じ接続でbuddy filterに切り替え (b/CALL* 形式, 30秒)...");
    tracing::info!("buddy設定: {:?}", buddies_with_wildcard);
    server
        .set_budlist_filter(buddies_with_wildcard.clone())
        .await
        .expect("buddy filter設定失敗");

    let mut count = 0;
    let mut received_from: std::collections::HashSet<String> = std::collections::HashSet::new();
    let deadline2 = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let remaining = deadline2.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, server.read_packet()).await {
            Ok(Ok(packet)) => {
                count += 1;
                match &packet {
                    AprsData::AprsPosition { callsign, .. } => {
                        tracing::info!("[buddy {}] Position: {}", count, callsign.callsign);
                        received_from.insert(callsign.callsign.clone());
                    }
                    AprsData::AprsMessage {
                        callsign, message, ..
                    } => {
                        tracing::info!(
                            "[buddy {}] Message: {} = {}",
                            count,
                            callsign.callsign,
                            message
                        );
                        received_from.insert(callsign.callsign.clone());
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::error!("エラー: {:?}", e);
                break;
            }
            Err(_) => {
                tracing::info!("30秒経過。終了。");
                break;
            }
        }
    }

    tracing::info!("=== Phase 3 結果 ===");
    tracing::info!("レンジフィルター受信パケット数: {}", range_count);
    tracing::info!("buddy filter受信パケット数: {}", count);
    tracing::info!("buddy設定: {:?}", live_callsigns);
    tracing::info!("buddy受信局: {:?}", received_from);

    if count == 0 {
        tracing::error!(
            "【確定】同一接続でレンジフィルターは動作するがbuddyフィルターは0パケット。\n\
            サーバー側の b/ filter実装に問題があります。"
        );
    } else {
        tracing::info!("buddy filterは正常動作しています。");
    }
}

/// フェーズ5: f/ (friend filter) のテスト
///
/// b/ buddy filterが壊れているサーバーで代替として f/ フィルターを検証。
/// f/CALLSIGN/DIST は CALLSIGN の最終既知位置から DIST km 以内のパケットを通す。
///
/// Step 1: r/ レンジフィルターで生きている局を収集
/// Step 2: 同一接続を f/ フィルターに切り替えてパケットが届くか確認
#[tokio::test]
#[ignore]
async fn phase5_friend_filter_from_live_stations() {
    load_env();
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init()
        .ok();

    let host = env::var("APRSHOST").unwrap_or_else(|_| "rotate.aprs2.net:14580".to_string());
    let user = env::var("APRSUSER").expect("APRSUSER not set");
    let password = env::var("APRSPASSWORD").expect("APRSPASSWORD not set");

    tracing::info!("=== Phase 5: f/ (friend filter) テスト ===");

    // [Step 1] レンジフィルターで生きている局を収集
    tracing::info!("[Step 1] レンジフィルターで局を収集中 (30秒)...");
    let server = AprsIS::connect(&host, &user, &password)
        .await
        .expect("APRS-IS接続失敗");
    server
        .set_filter("r/35.68/139.77/500".to_string())
        .await
        .expect("レンジフィルター設定失敗");

    let mut callsign_count: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    let deadline1 = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut range_count = 0;
    loop {
        let remaining = deadline1.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, server.read_packet()).await {
            Ok(Ok(AprsData::AprsPosition { callsign, .. })) => {
                range_count += 1;
                let cnt = callsign_count.entry(callsign.callsign.clone()).or_insert(0);
                *cnt += 1;
                if *cnt == 1 {
                    tracing::info!("[range {}] 初出現: {}", range_count, callsign.callsign);
                } else {
                    tracing::info!(
                        "[range {}] 再出現({}回目): {}",
                        range_count,
                        cnt,
                        callsign.callsign
                    );
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    // 複数回出現した局を優先（頻繁にビーコンしている局）
    let mut frequent: Vec<String> = callsign_count
        .iter()
        .filter(|(_, &v)| v >= 2)
        .map(|(k, _)| k.clone())
        .collect();
    let mut once: Vec<String> = callsign_count
        .iter()
        .filter(|(_, &v)| v == 1)
        .map(|(k, _)| k.clone())
        .collect();
    frequent.sort();
    once.sort();
    let live_callsigns: Vec<String> = {
        let mut v = frequent;
        v.extend(once.into_iter().take(5usize.saturating_sub(v.len())));
        v.truncate(5);
        v
    };
    tracing::info!("選択局 ({} 局): {:?}", live_callsigns.len(), live_callsigns);
    assert!(
        !live_callsigns.is_empty(),
        "レンジフィルターで局を収集できませんでした"
    );

    // [Step 2] 同一接続で f/ フィルターに切り替え
    // f/CALLSIGN/DIST: CALLSIGNの最終既知位置からDIST km以内のパケットを通す
    // 半径200kmにすれば山頂周辺のパケットも十分カバーできる
    let friend_filter = live_callsigns
        .iter()
        .map(|c| format!("f/{}/200", c))
        .collect::<Vec<_>>()
        .join(" ");
    tracing::info!("[Step 2] 同じ接続で f/ フィルターに切り替え (30秒)...");
    tracing::info!("フィルター文字列: {}", friend_filter);
    server
        .set_filter(friend_filter.clone())
        .await
        .expect("f/ フィルター設定失敗");

    let mut count = 0;
    let mut received_from: std::collections::HashSet<String> = std::collections::HashSet::new();
    let deadline2 = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let remaining = deadline2.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, server.read_packet()).await {
            Ok(Ok(packet)) => {
                count += 1;
                match &packet {
                    AprsData::AprsPosition { callsign, .. } => {
                        tracing::info!("[friend {}] Position: {}", count, callsign.callsign);
                        received_from.insert(callsign.callsign.clone());
                    }
                    AprsData::AprsMessage {
                        callsign, message, ..
                    } => {
                        tracing::info!(
                            "[friend {}] Message: {} = {}",
                            count,
                            callsign.callsign,
                            message
                        );
                        received_from.insert(callsign.callsign.clone());
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::error!("エラー: {:?}", e);
                break;
            }
            Err(_) => {
                tracing::info!("30秒経過。終了。");
                break;
            }
        }
    }

    tracing::info!("=== Phase 5 結果 ===");
    tracing::info!("レンジフィルター受信パケット数: {}", range_count);
    tracing::info!("f/ フィルター受信パケット数: {}", count);
    tracing::info!("f/ フィルター設定: {}", friend_filter);
    tracing::info!("受信局: {:?}", received_from);

    if count == 0 {
        tracing::error!(
            "【f/ フィルターも動作しない】b/ と同様に 0 パケット。\n\
            サーバー側のフィルター実装が全般的に問題の可能性。\n\
            r/ レンジフィルター（日本全体 r/35/135/1000 等）への切り替えを検討。"
        );
    } else {
        tracing::info!(
            "【f/ フィルターは動作する】b/ の代替として利用可能。\n\
            SOTAアラート局を f/CALLSIGN/200 でフィルターする方式に変更可能。"
        );
    }
}

/// フェーズ6: r/ + t/p/m タイプフィルターのテスト
///
/// b/ と f/ が壊れているサーバーで r/ レンジ + t/ タイプフィルターを組み合わせ、
/// position と message のみを受信できるか確認する。
///
/// 動作確認後は本体の set_buddy_list を r/+t/p/m に変更し、
/// アプリ側でコールサインフィルタリングを行う実装に切り替える。
#[tokio::test]
#[ignore]
async fn phase6_range_with_type_filter() {
    load_env();
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init()
        .ok();

    let host = env::var("APRSHOST").unwrap_or_else(|_| "rotate.aprs2.net:14580".to_string());
    let user = env::var("APRSUSER").expect("APRSUSER not set");
    let password = env::var("APRSPASSWORD").expect("APRSPASSWORD not set");

    tracing::info!("=== Phase 6: r/ + t/p/m タイプフィルターテスト ===");

    let server = AprsIS::connect(&host, &user, &password)
        .await
        .expect("APRS-IS接続失敗");

    // 日本全体をカバーする範囲 + position/message のみ
    let filter = "r/36/137/1500 t/p/m".to_string();
    tracing::info!("フィルター設定: {}", filter);
    server
        .set_filter(filter.clone())
        .await
        .expect("フィルター設定失敗");

    let mut count = 0;
    let mut pos_count = 0;
    let mut msg_count = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, server.read_packet()).await {
            Ok(Ok(packet)) => {
                count += 1;
                match &packet {
                    AprsData::AprsPosition {
                        callsign,
                        latitude,
                        longitude,
                    } => {
                        pos_count += 1;
                        tracing::info!(
                            "[{} pos] {}: ({:.4}, {:.4})",
                            count,
                            callsign.callsign,
                            latitude,
                            longitude
                        );
                    }
                    AprsData::AprsMessage {
                        callsign,
                        addressee,
                        message,
                    } => {
                        msg_count += 1;
                        tracing::info!(
                            "[{} msg] {} -> {}: {}",
                            count,
                            callsign.callsign,
                            addressee,
                            message
                        );
                    }
                }
                if count >= 10 {
                    break;
                }
            }
            Ok(Err(e)) => {
                tracing::error!("エラー: {:?}", e);
                break;
            }
            Err(_) => {
                tracing::info!("30秒経過。終了。");
                break;
            }
        }
    }

    tracing::info!("=== Phase 6 結果 ===");
    tracing::info!("フィルター: {}", filter);
    tracing::info!(
        "合計受信: {} パケット (position: {}, message: {})",
        count,
        pos_count,
        msg_count
    );

    if count > 0 {
        tracing::info!("【成功】r/ + t/p/m フィルターは動作します。本体実装を変更可能。");
    } else {
        tracing::error!("【失敗】r/ + t/p/m フィルターも動作しません。");
    }

    assert!(count > 0, "r/ + t/p/m フィルターでパケットが届きません");
}

/// フェーズ4: 自局をbuddyに設定し、別の端末からビーコンを送信して受信確認
///
/// 実際の運用と同じbuddyフィルターを使い、自分でテストパケットを送って確認。
/// APRSUSER の基本コールサイン（SSID除去）をbuddyに設定。
#[tokio::test]
#[ignore]
async fn phase4_self_beacon_buddy_test() {
    load_env();
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();

    let host = env::var("APRSHOST").unwrap_or_else(|_| "rotate.aprs2.net:14580".to_string());
    let user = env::var("APRSUSER").expect("APRSUSER not set");
    let password = env::var("APRSPASSWORD").expect("APRSPASSWORD not set");

    // JL1NIE-10 -> JL1NIE
    let base_callsign = user
        .rsplit_once('-')
        .map(|(call, _)| call.to_string())
        .unwrap_or(user.clone());

    tracing::info!("=== Phase 4: 自局buddy filterテスト ===");
    tracing::info!("buddy: {}", base_callsign);

    let server = AprsIS::connect(&host, &user, &password)
        .await
        .expect("APRS-IS接続失敗");

    server
        .set_budlist_filter(vec![base_callsign.clone()])
        .await
        .expect("buddy filter設定失敗");

    tracing::info!("buddy filter設定完了: b/{}", base_callsign);
    tracing::info!(
        "別の端末（Winlink/APRSDroid等）から {} のSSIDでビーコンを送信してください",
        base_callsign
    );
    tracing::info!("120秒間待機...");

    let mut count = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, server.read_packet()).await {
            Ok(Ok(packet)) => {
                count += 1;
                tracing::info!("[{}] パケット受信: {:?}", count, packet);
            }
            Ok(Err(e)) => {
                tracing::error!("エラー: {:?}", e);
                break;
            }
            Err(_) => {
                tracing::info!("120秒経過。待機終了。");
                break;
            }
        }
    }

    tracing::info!("=== Phase 4 結果: {}パケット受信 ===", count);
}
