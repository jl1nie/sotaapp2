//! WSPR SVG生成サービス
//!
//! WSPRスポットデータからSVG散布図を生成する

use std::collections::{HashMap, HashSet};

use chrono::{NaiveDateTime, TimeZone, Utc};
use plotters::prelude::*;
use serde::Deserialize;

/// WSPRリクエストのJSON構造
#[derive(Debug, Deserialize)]
pub struct WsprRequest {
    pub title: String,
    pub plots: Vec<PlotConfig>,
    pub spots: String,
    pub min: i32,
    pub max: i32,
    pub label: bool,
    pub width: i32,
}

/// プロット設定
#[derive(Debug, Deserialize)]
pub struct PlotConfig {
    pub label: String,
    pub color: String,
    pub from: String,
    pub to: String,
}

/// 内部プロットデータ
#[derive(Debug, Clone)]
struct PlotData {
    label: String,
    color: RGBColor,
    from: i64,
    to: i64,
    repo: Vec<String>,
    dist: Vec<i32>,
    snr: Vec<i32>,
    avgdist: Vec<i32>,
    avgsnr: Vec<f64>,
}

/// WSPRスポットデータ
#[derive(Debug)]
struct WsprSpot {
    ts: i64,
    snr: i32,
    repo: String,
}

/// レポーター情報
#[derive(Debug)]
struct Reporter {
    distance: i32,
    #[allow(dead_code)]
    azimuth: i32,
}

/// 共通レポーター情報
struct CommonReporter {
    dist: i32,
    snr: Vec<Vec<i32>>,
}

/// 日時文字列をタイムスタンプに変換
fn parse_datetime(s: &str) -> Option<i64> {
    // Format: "YYYY-MM-DD HH:MM" or "YYYY/MM/DD HH:MM:SS"
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        return Some(Utc.from_utc_datetime(&dt).timestamp());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M:%S") {
        return Some(Utc.from_utc_datetime(&dt).timestamp());
    }
    None
}

/// カラー文字列をRGBColorに変換
fn parse_color(color: &str) -> RGBColor {
    let color = color.trim_start_matches('#');
    if color.len() >= 6 {
        let r = u8::from_str_radix(&color[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&color[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&color[4..6], 16).unwrap_or(0);
        RGBColor(r, g, b)
    } else {
        // カラー名をサポート
        match color.to_lowercase().as_str() {
            "red" => RED,
            "blue" => BLUE,
            "green" => GREEN,
            "black" => BLACK,
            "yellow" => YELLOW,
            "cyan" => CYAN,
            "magenta" => MAGENTA,
            "orange" => RGBColor(255, 165, 0),
            "purple" => RGBColor(128, 0, 128),
            _ => BLACK,
        }
    }
}

/// WSPRスポットからSVGグラフを生成
pub fn generate_wspr_svg(request: &WsprRequest) -> Result<String, String> {
    // プロットデータの初期化
    let mut plots: Vec<PlotData> = request
        .plots
        .iter()
        .filter_map(|p| {
            let from = parse_datetime(&p.from)?;
            let to = parse_datetime(&p.to)?;
            Some(PlotData {
                label: p.label.clone(),
                color: parse_color(&p.color),
                from,
                to,
                repo: Vec::new(),
                dist: Vec::new(),
                snr: Vec::new(),
                avgdist: Vec::new(),
                avgsnr: Vec::new(),
            })
        })
        .collect();

    plots.sort_by_key(|p| p.from);

    // スポットデータのパース
    let mut wspr_spots: Vec<WsprSpot> = Vec::new();
    let mut reporters: HashMap<String, Reporter> = HashMap::new();

    for line in request.spots.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() == 13 {
            let datetime_str = format!("{} {}", cols[0], cols[1]);
            if let Some(ts) = parse_datetime(&datetime_str) {
                let snr: i32 = cols[4].parse().unwrap_or(0);
                let rp = cols[8].to_string();
                let km: i32 = cols[10].parse().unwrap_or(0);
                let az: i32 = cols[11].parse().unwrap_or(0);

                reporters.insert(
                    rp.clone(),
                    Reporter {
                        distance: km,
                        azimuth: az,
                    },
                );

                if km >= request.min && km <= request.max {
                    wspr_spots.push(WsprSpot { ts, snr, repo: rp });
                }
            }
        }
    }

    wspr_spots.sort_by_key(|s| s.ts);

    // スポットをプロットに振り分け
    let mut plot_idx = 0;
    for sp in &wspr_spots {
        while plot_idx < plots.len() {
            if sp.ts >= plots[plot_idx].from {
                if sp.ts <= plots[plot_idx].to {
                    if let Some(reporter) = reporters.get(&sp.repo) {
                        plots[plot_idx].snr.push(sp.snr);
                        plots[plot_idx].repo.push(sp.repo.clone());
                        plots[plot_idx].dist.push(reporter.distance);
                    }
                    break;
                } else {
                    plot_idx += 1;
                }
            } else {
                break;
            }
        }
        if plot_idx >= plots.len() {
            break;
        }
    }

    // 共通レポーターの計算
    let mut common_reporters: Option<HashSet<String>> = None;
    for p in &plots {
        let repo_set: HashSet<String> = p.repo.iter().cloned().collect();
        common_reporters = Some(match common_reporters {
            Some(set) => set.intersection(&repo_set).cloned().collect(),
            None => repo_set,
        });
    }
    let common_reporters = common_reporters.unwrap_or_default();

    let mut common_reporter_data: HashMap<String, CommonReporter> = common_reporters
        .iter()
        .filter_map(|stn| {
            reporters.get(stn).map(|r| {
                (
                    stn.clone(),
                    CommonReporter {
                        dist: r.distance,
                        snr: vec![Vec::new(); plots.len()],
                    },
                )
            })
        })
        .collect();

    // 共通レポーターのSNRを収集
    plot_idx = 0;
    for sp in &wspr_spots {
        while plot_idx < plots.len() {
            if sp.ts >= plots[plot_idx].from {
                if sp.ts <= plots[plot_idx].to {
                    if common_reporters.contains(&sp.repo) {
                        if let Some(cr) = common_reporter_data.get_mut(&sp.repo) {
                            cr.snr[plot_idx].push(sp.snr);
                        }
                    }
                    break;
                } else {
                    plot_idx += 1;
                }
            } else {
                break;
            }
        }
        if plot_idx >= plots.len() {
            break;
        }
    }

    // 距離でソートした共通レポーターリスト
    let mut common_list: Vec<&String> = common_reporter_data.keys().collect();
    common_list.sort_by_key(|k| common_reporter_data.get(*k).map(|c| c.dist).unwrap_or(0));

    // 平均SNRの計算
    for rp in &common_list {
        if let Some(cr) = common_reporter_data.get(*rp) {
            for (j, snr_list) in cr.snr.iter().enumerate().take(plot_idx) {
                if j < plots.len() {
                    let avg = if !snr_list.is_empty() {
                        snr_list.iter().sum::<i32>() as f64 / snr_list.len() as f64
                    } else {
                        0.0
                    };
                    plots[j].avgsnr.push(avg);
                    plots[j].avgdist.push(cr.dist);
                }
            }
        }
    }

    // データの範囲を計算
    let (min_dist, max_dist, min_snr, max_snr) = {
        let mut min_d = i32::MAX;
        let mut max_d = i32::MIN;
        let mut min_s = i32::MAX;
        let mut max_s = i32::MIN;

        for p in &plots {
            for &d in &p.dist {
                min_d = min_d.min(d);
                max_d = max_d.max(d);
            }
            for &s in &p.snr {
                min_s = min_s.min(s);
                max_s = max_s.max(s);
            }
        }

        // デフォルト値
        if min_d == i32::MAX {
            min_d = 0;
        }
        if max_d == i32::MIN {
            max_d = 10000;
        }
        if min_s == i32::MAX {
            min_s = -30;
        }
        if max_s == i32::MIN {
            max_s = 10;
        }

        // マージンを追加
        let margin_d = ((max_d - min_d) as f64 * 0.1) as i32;
        let margin_s = ((max_s - min_s) as f64 * 0.1) as i32;

        (
            min_d - margin_d,
            max_d + margin_d,
            min_s - margin_s,
            max_s + margin_s,
        )
    };

    // SVG生成
    let width = (request.width as u32).max(400);
    let height = (width as f32 * 0.6) as u32;

    let mut svg_buffer = String::new();
    {
        let root = SVGBackend::with_string(&mut svg_buffer, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;

        let mut chart = ChartBuilder::on(&root)
            .caption(&request.title, ("sans-serif", 20).into_font())
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(min_dist..max_dist, min_snr..max_snr)
            .map_err(|e| e.to_string())?;

        chart
            .configure_mesh()
            .x_desc("Distance (km)")
            .y_desc("SNR (dB)")
            .draw()
            .map_err(|e| e.to_string())?;

        // プロットを描画
        for p in &plots {
            let label = format!("{} {}spots", p.label, p.snr.len());

            // 散布図
            let points: Vec<(i32, i32)> = p
                .dist
                .iter()
                .zip(p.snr.iter())
                .map(|(&d, &s)| (d, s))
                .collect();

            chart
                .draw_series(
                    points
                        .iter()
                        .map(|&(x, y)| Circle::new((x, y), 4, p.color.filled())),
                )
                .map_err(|e| e.to_string())?
                .label(label)
                .legend(move |(x, y)| Circle::new((x, y), 4, p.color.filled()));

            // 平均線（薄い色で）
            if !p.avgdist.is_empty() {
                let avg_points: Vec<(i32, i32)> = p
                    .avgdist
                    .iter()
                    .zip(p.avgsnr.iter())
                    .map(|(&d, &s)| (d, s as i32))
                    .collect();

                let light_color = RGBColor(
                    (p.color.0 as u16 + 200).min(255) as u8,
                    (p.color.1 as u16 + 200).min(255) as u8,
                    (p.color.2 as u16 + 200).min(255) as u8,
                );

                chart
                    .draw_series(LineSeries::new(
                        avg_points.clone(),
                        light_color.stroke_width(1),
                    ))
                    .map_err(|e| e.to_string())?;

                chart
                    .draw_series(
                        avg_points
                            .iter()
                            .map(|&(x, y)| Cross::new((x, y), 3, light_color)),
                    )
                    .map_err(|e| e.to_string())?;
            }

            // ラベル表示
            if request.label {
                for (i, label) in p.repo.iter().enumerate() {
                    if i < p.dist.len() && i < p.snr.len() {
                        chart
                            .draw_series(std::iter::once(Text::new(
                                label.clone(),
                                (p.dist[i], p.snr[i]),
                                ("sans-serif", 10).into_font(),
                            )))
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }

        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .position(SeriesLabelPosition::UpperRight)
            .draw()
            .map_err(|e| e.to_string())?;

        root.present().map_err(|e| e.to_string())?;
    }

    Ok(svg_buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        let ts1 = parse_datetime("2024-01-15 12:30").unwrap();
        assert!(ts1 > 0);

        let ts2 = parse_datetime("2024/01/15 12:30:00").unwrap();
        assert!(ts2 > 0);

        assert!(parse_datetime("invalid").is_none());
    }

    #[test]
    fn test_parse_color() {
        let red = parse_color("#ff0000");
        assert_eq!(red, RGBColor(255, 0, 0));

        let blue = parse_color("blue");
        assert_eq!(blue, BLUE);
    }

    #[test]
    fn test_generate_empty_svg() {
        let request = WsprRequest {
            title: "Test".to_string(),
            plots: vec![PlotConfig {
                label: "Plot1".to_string(),
                color: "red".to_string(),
                from: "2024-01-01 00:00".to_string(),
                to: "2024-01-01 23:59".to_string(),
            }],
            spots: String::new(),
            min: 0,
            max: 10000,
            label: false,
            width: 800,
        };

        let result = generate_wspr_svg(&request);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Test"));
    }
}
