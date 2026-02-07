//! PDF生成テスト（バックエンドロジック使用）
//!
//! Run with: cargo run --example test_pdf_gen --package service

use common::award_config::{AwardTemplateConfig, TemplateConfig, TextOverlayConfig};
use service::implement::award_calculator::{detect_log_type, judge_award_with_mode};
use service::implement::award_pdf::{AwardPdfGenerator, AwardType, CertificateInfo};
use service::model::award::{AwardPeriod, JudgmentMode, LogType, SotaLogEntry};
use std::fs;

fn parse_csv(csv_data: &str) -> Vec<SotaLogEntry> {
    let mut logs = Vec::new();
    for line in csv_data.lines() {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() >= 10 {
            logs.push(SotaLogEntry {
                version: fields[0].to_string(),
                my_callsign: fields[1].to_string(),
                my_summit_code: if fields[2].is_empty() {
                    None
                } else {
                    Some(fields[2].to_string())
                },
                date: fields[3].to_string(),
                time: fields[4].to_string(),
                frequency: fields[5].to_string(),
                mode: fields[6].to_string(),
                his_callsign: fields[7].to_string(),
                his_summit_code: if fields.len() > 8 && !fields[8].is_empty() {
                    Some(fields[8].to_string())
                } else {
                    None
                },
                comment: if fields.len() > 9 {
                    Some(fields[9].to_string())
                } else {
                    None
                },
            });
        }
    }
    logs
}

fn process_log(csv_path: &str, template_dir: &str, config: &AwardTemplateConfig) {
    println!("\n============================================================");
    println!("Processing: {}", csv_path);
    println!("============================================================");

    // CSVファイルを読み込み
    let csv_data = match fs::read_to_string(csv_path) {
        Ok(data) => data,
        Err(e) => {
            println!("CSVファイルを読み込めません: {}", e);
            return;
        }
    };

    // ログ種別を判定
    let log_type = detect_log_type(&csv_data);
    println!("Log type: {:?}", log_type);

    if log_type == LogType::Unknown {
        println!("Unknown log type, skipping...");
        return;
    }

    // CSVをパース
    let logs = parse_csv(&csv_data);
    println!("Total log entries: {}", logs.len());

    // アワード期間を設定
    let period = AwardPeriod::default();
    println!(
        "Award period: {} to {}",
        period.start.format("%Y-%m-%d"),
        period.end.format("%Y-%m-%d")
    );

    // アワード判定を実行
    let result = judge_award_with_mode(logs, &period, JudgmentMode::Strict, log_type);

    println!("\n=== Award Result ===");
    println!("Callsign: {}", result.callsign);
    println!("Total QSOs in period: {}", result.total_qsos);

    // アクティベータ賞の処理
    if let Some(ref activator) = result.activator {
        println!("\n--- Activator Award ---");
        println!("Achieved: {}", activator.achieved);
        println!("Qualified summits: {}", activator.qualified_summits);
        println!("Top 10 summits:");
        for (i, summit) in activator.summits.iter().take(10).enumerate() {
            println!(
                "  {}. {} - {} stations ({})",
                i + 1,
                summit.summit_code,
                summit.unique_stations,
                if summit.qualified {
                    "QUALIFIED"
                } else {
                    "not qualified"
                }
            );
        }

        if activator.achieved {
            let generator = AwardPdfGenerator::new(template_dir.to_string(), config.clone());
            let info = CertificateInfo {
                callsign: result.callsign.clone(),
                achievement_text: format!("{} Summits Activated", activator.qualified_summits),
                achievement_line2: None,
                description: Some(
                    "SOTA Japan Branch 10th Anniversary Award - Activator".to_string(),
                ),
                issue_date: Some("2026 Feb. 1".to_string()),
            };

            match generator.generate(AwardType::Activator, &info) {
                Ok(pdf_data) => {
                    let output_path = format!(
                        "/mnt/c/Users/minor/OneDrive/デスクトップ/images/{}_activator_award.pdf",
                        result.callsign
                    );
                    fs::write(&output_path, &pdf_data).unwrap();
                    println!(
                        "\nPDF generated: {} ({} bytes)",
                        output_path,
                        pdf_data.len()
                    );
                }
                Err(e) => {
                    println!("Error generating PDF: {:?}", e);
                }
            }
        }
    }

    // チェイサー賞の処理
    if let Some(ref chaser) = result.chaser {
        println!("\n--- Chaser Award ---");
        println!("Achieved: {}", chaser.achieved);
        println!("Qualified summits: {}", chaser.qualified_summits.len());
        println!("Top 10 summits:");
        for (i, summit) in chaser.qualified_summits.iter().take(10).enumerate() {
            println!(
                "  {}. {} - {} activators",
                i + 1,
                summit.summit_code,
                summit.unique_activators,
            );
        }

        if chaser.achieved {
            let generator = AwardPdfGenerator::new(template_dir.to_string(), config.clone());
            // チェイサー賞: 達成サミット数と最大アクティベータ数を表示
            let qualified_count = chaser.qualified_summits.len();
            let best_summit = &chaser.qualified_summits[0];
            let info = CertificateInfo {
                callsign: result.callsign.clone(),
                achievement_text: format!("{} Summits Chased", qualified_count),
                achievement_line2: Some(format!(
                    "Best: {} from {}",
                    best_summit.unique_activators, best_summit.summit_code
                )),
                description: Some("SOTA Japan Branch 10th Anniversary Award - Chaser".to_string()),
                issue_date: Some("2026 Feb. 1".to_string()),
            };

            match generator.generate(AwardType::Chaser, &info) {
                Ok(pdf_data) => {
                    let output_path = format!(
                        "/mnt/c/Users/minor/OneDrive/デスクトップ/images/{}_chaser_award.pdf",
                        result.callsign
                    );
                    fs::write(&output_path, &pdf_data).unwrap();
                    println!(
                        "\nPDF generated: {} ({} bytes)",
                        output_path,
                        pdf_data.len()
                    );
                }
                Err(e) => {
                    println!("Error generating PDF: {:?}", e);
                }
            }
        }
    }
}

fn main() {
    // テンプレートディレクトリを設定
    let template_dir = "/tmp/award_templates";
    fs::create_dir_all(template_dir).unwrap();

    // JPG画像をテンプレートディレクトリにコピー（両方同じ画像を使用）
    let src_jpg = "/mnt/c/Users/minor/OneDrive/デスクトップ/images/2.jpg";
    let activator_jpg = format!("{}/activator_template.jpg", template_dir);
    let chaser_jpg = format!("{}/chaser_template.jpg", template_dir);
    fs::copy(src_jpg, &activator_jpg).unwrap();
    fs::copy(src_jpg, &chaser_jpg).unwrap();
    println!("Template copied to: {}", template_dir);

    // A4横向き用の設定
    let config = AwardTemplateConfig {
        activator: TemplateConfig {
            callsign: TextOverlayConfig {
                x: 420.0,
                y: 500.0,
                font_size: 72.0,
                color: [255, 0, 0], // 赤
                centered: true,
            },
            achievement: TextOverlayConfig {
                x: 420.0,
                y: 420.0,
                font_size: 32.0,
                color: [255, 0, 0], // 赤
                centered: true,
            },
            issue_date: TextOverlayConfig {
                x: 420.0,
                y: 120.0,
                font_size: 14.0,
                color: [255, 0, 0],
                centered: true,
            },
        },
        chaser: TemplateConfig {
            callsign: TextOverlayConfig {
                x: 420.0,
                y: 500.0,
                font_size: 72.0,
                color: [255, 0, 0], // 赤
                centered: true,
            },
            achievement: TextOverlayConfig {
                x: 420.0,
                y: 420.0,
                font_size: 32.0,
                color: [255, 0, 0], // 赤
                centered: true,
            },
            issue_date: TextOverlayConfig {
                x: 420.0,
                y: 120.0,
                font_size: 14.0,
                color: [255, 0, 0],
                centered: true,
            },
        },
    };

    // アクティベータログを処理
    process_log(
        "/mnt/c/Users/minor/OneDrive/デスクトップ/images/JG0AWE_activator_20260207.csv",
        template_dir,
        &config,
    );

    // チェイサーログを処理
    process_log(
        "/mnt/c/Users/minor/OneDrive/デスクトップ/images/JG0AWE_chaser_20260207.csv",
        template_dir,
        &config,
    );

    // JA1VVHのチェイサーログを処理
    process_log(
        "/mnt/c/Users/minor/OneDrive/デスクトップ/images/JA1VVH_chaser_20260207.csv",
        template_dir,
        &config,
    );

    println!("\n=== Done ===");
}
