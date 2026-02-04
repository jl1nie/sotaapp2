//! アワード証明書テンプレート設定

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// テキストオーバーレイの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOverlayConfig {
    /// X座標（ポイント、左端からの距離）
    pub x: f32,
    /// Y座標（ポイント、下端からの距離）
    pub y: f32,
    /// フォントサイズ
    pub font_size: f32,
    /// 色（R, G, B、0-255）
    pub color: [u8; 3],
}

impl Default for TextOverlayConfig {
    fn default() -> Self {
        Self {
            x: 297.0, // A4中央付近
            y: 400.0,
            font_size: 24.0,
            color: [0, 0, 0], // 黒
        }
    }
}

/// 単一テンプレートの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// コールサインの印字設定
    pub callsign: TextOverlayConfig,
    /// 達成内容の印字設定
    pub achievement: TextOverlayConfig,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            callsign: TextOverlayConfig {
                x: 297.0,
                y: 450.0,
                font_size: 36.0,
                color: [0, 0, 128], // ネイビー
            },
            achievement: TextOverlayConfig {
                x: 297.0,
                y: 380.0,
                font_size: 18.0,
                color: [0, 0, 0],
            },
        }
    }
}

/// アワードテンプレート全体の設定
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AwardTemplateConfig {
    /// アクティベータ賞テンプレート設定
    pub activator: TemplateConfig,
    /// チェイサー賞テンプレート設定
    pub chaser: TemplateConfig,
}

impl AwardTemplateConfig {
    /// 設定ファイルから読み込み
    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            // ファイルがなければデフォルト設定を作成して保存
            let config = Self::default();
            config.save_to_file(path)?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("設定ファイルの読み込みに失敗: {:?}", path))?;

        serde_json::from_str(&content)
            .with_context(|| format!("設定ファイルのパースに失敗: {:?}", path))
    }

    /// 設定ファイルに保存
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // 親ディレクトリがなければ作成
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("ディレクトリの作成に失敗: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(self).context("設定のシリアライズに失敗")?;

        std::fs::write(path, content)
            .with_context(|| format!("設定ファイルの書き込みに失敗: {:?}", path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = AwardTemplateConfig::default();
        assert_eq!(config.activator.callsign.font_size, 36.0);
        assert_eq!(config.chaser.callsign.color, [0, 0, 128]);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("award_config.json");

        let config = AwardTemplateConfig::default();
        config.save_to_file(&path).unwrap();

        let loaded = AwardTemplateConfig::load_from_file(&path).unwrap();
        assert_eq!(
            loaded.activator.callsign.font_size,
            config.activator.callsign.font_size
        );
    }

    #[test]
    fn test_load_creates_default_if_not_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("new_config.json");

        assert!(!path.exists());
        let config = AwardTemplateConfig::load_from_file(&path).unwrap();
        assert!(path.exists());
        assert_eq!(config.activator.callsign.font_size, 36.0);
    }
}
