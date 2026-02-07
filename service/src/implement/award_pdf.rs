//! アワード証明書PDF生成（画像テンプレート方式）

use anyhow::{Context, Result};
use common::award_config::{AwardTemplateConfig, TemplateConfig};
use printpdf::*;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

// printpdfと名前衝突を避けるため外部クレートを明示
use ::image as img_crate;

/// A4サイズ（横向き）の定数
const A4_WIDTH_MM: f32 = 297.0;
const A4_HEIGHT_MM: f32 = 210.0;

/// アワードの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwardType {
    Activator,
    Chaser,
}

impl AwardType {
    /// テンプレート画像ファイル名（JPG/PNG対応）
    pub fn template_filename(&self) -> &'static str {
        match self {
            AwardType::Activator => "activator_template",
            AwardType::Chaser => "chaser_template",
        }
    }
}

/// アワード証明書の生成情報
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub callsign: String,
    pub achievement_text: String,
    /// 達成内容の2行目（Bestなど）
    pub achievement_line2: Option<String>,
    /// 達成内容の説明（小さいフォントで表示）
    pub description: Option<String>,
    /// 発行日（例: "2026 Feb. 1"）
    pub issue_date: Option<String>,
}

/// アワードPDF生成器
pub struct AwardPdfGenerator {
    template_dir: String,
    config: AwardTemplateConfig,
}

impl AwardPdfGenerator {
    pub fn new(template_dir: String, config: AwardTemplateConfig) -> Self {
        Self {
            template_dir,
            config,
        }
    }

    /// テンプレート画像のパスを取得（JPG/PNGを自動検出）
    fn find_template_path(&self, award_type: AwardType) -> Option<std::path::PathBuf> {
        let base_name = award_type.template_filename();
        let dir = Path::new(&self.template_dir);

        // PNG, JPG, JPEG の順で探す
        for ext in &["png", "jpg", "jpeg"] {
            let path = dir.join(format!("{}.{}", base_name, ext));
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    /// テンプレートが存在するかチェック
    pub fn template_exists(&self, award_type: AwardType) -> bool {
        self.find_template_path(award_type).is_some()
    }

    /// PDF証明書を生成
    /// A4横向き（297mm x 210mm）で固定し、画像をフィットさせる
    pub fn generate(&self, award_type: AwardType, info: &CertificateInfo) -> Result<Vec<u8>> {
        let template_path = self
            .find_template_path(award_type)
            .context("テンプレート画像が見つかりません")?;

        let template_config = match award_type {
            AwardType::Activator => &self.config.activator,
            AwardType::Chaser => &self.config.chaser,
        };

        // 画像を読み込み
        let img = self.load_image(&template_path)?;

        // A4横向きでPDFを作成
        let (doc, page1, layer1) = PdfDocument::new(
            "SOTA 10th Anniversary Award Certificate",
            Mm(A4_WIDTH_MM),
            Mm(A4_HEIGHT_MM),
            "Background",
        );

        let current_layer = doc.get_page(page1).get_layer(layer1);

        // 背景画像をA4サイズにフィットさせて配置
        self.add_background_image(&current_layer, &img, A4_WIDTH_MM, A4_HEIGHT_MM)?;

        // テキストレイヤーを追加
        let text_layer = doc.get_page(page1).add_layer("Text");
        self.add_text_overlay(&doc, &text_layer, info, template_config)?;

        // PDFをバイト列として出力
        let mut buffer = Vec::new();
        doc.save(&mut std::io::BufWriter::new(&mut buffer))
            .context("PDFの保存に失敗")?;

        Ok(buffer)
    }

    fn load_image(&self, path: &Path) -> Result<img_crate::DynamicImage> {
        let file =
            File::open(path).with_context(|| format!("画像ファイルを開けません: {:?}", path))?;
        let reader = BufReader::new(file);

        img_crate::load(
            reader,
            img_crate::ImageFormat::from_path(path)
                .with_context(|| format!("画像形式を判定できません: {:?}", path))?,
        )
        .with_context(|| format!("画像の読み込みに失敗: {:?}", path))
    }

    fn add_background_image(
        &self,
        layer: &PdfLayerReference,
        img: &img_crate::DynamicImage,
        page_width_mm: f32,
        page_height_mm: f32,
    ) -> Result<()> {
        // DynamicImage を printpdf の Image に変換（embedded_images featureが必要）
        let pdf_image = Image::from_dynamic_image(img);

        let (img_width, img_height) = (img.width() as f32, img.height() as f32);

        // ページサイズをポイント単位に変換
        let page_width_pt = page_width_mm / 25.4 * 72.0;
        let page_height_pt = page_height_mm / 25.4 * 72.0;

        // 画像のアスペクト比を維持しながらページにフィットさせる
        let img_aspect = img_width / img_height;
        let page_aspect = page_width_mm / page_height_mm;

        let (scale, offset_x, offset_y) = if img_aspect > page_aspect {
            // 画像が横長の場合：幅に合わせてスケール
            let scale = page_width_pt / img_width;
            let scaled_height = img_height * scale;
            let offset_y = (page_height_pt - scaled_height) / 2.0;
            (scale, 0.0, offset_y)
        } else {
            // 画像が縦長の場合：高さに合わせてスケール
            let scale = page_height_pt / img_height;
            let scaled_width = img_width * scale;
            let offset_x = (page_width_pt - scaled_width) / 2.0;
            (scale, offset_x, 0.0)
        };

        // 画像をページにフィットさせて中央配置
        pdf_image.add_to_layer(
            layer.clone(),
            ImageTransform {
                translate_x: Some(Mm(offset_x / 72.0 * 25.4)),
                translate_y: Some(Mm(offset_y / 72.0 * 25.4)),
                scale_x: Some(scale),
                scale_y: Some(scale),
                dpi: Some(72.0),
                ..Default::default()
            },
        );

        Ok(())
    }

    /// Helvetica Boldの文字幅（1000単位/em）
    /// 参照: Adobe Helvetica Bold AFM
    fn char_width_helvetica_bold(c: char) -> u32 {
        match c {
            // 数字
            '0'..='9' => 556,
            // 大文字
            'A' => 722,
            'B' => 722,
            'C' => 722,
            'D' => 722,
            'E' => 667,
            'F' => 611,
            'G' => 778,
            'H' => 722,
            'I' => 278,
            'J' => 556,
            'K' => 722,
            'L' => 611,
            'M' => 833,
            'N' => 722,
            'O' => 778,
            'P' => 667,
            'Q' => 778,
            'R' => 722,
            'S' => 667,
            'T' => 611,
            'U' => 722,
            'V' => 667,
            'W' => 944,
            'X' => 667,
            'Y' => 667,
            'Z' => 611,
            // 小文字
            'a' => 556,
            'b' => 611,
            'c' => 556,
            'd' => 611,
            'e' => 556,
            'f' => 333,
            'g' => 611,
            'h' => 611,
            'i' => 278,
            'j' => 278,
            'k' => 556,
            'l' => 278,
            'm' => 889,
            'n' => 611,
            'o' => 611,
            'p' => 611,
            'q' => 611,
            'r' => 389,
            's' => 556,
            't' => 333,
            'u' => 611,
            'v' => 556,
            'w' => 778,
            'x' => 556,
            'y' => 556,
            'z' => 500,
            // 記号
            ' ' => 278,
            '(' | ')' => 333,
            '+' => 584,
            '-' => 333,
            '/' => 278,
            // その他はデフォルト
            _ => 556,
        }
    }

    /// テキスト幅を計算（Helvetica Bold用）
    fn estimate_text_width(text: &str, font_size: f32, _is_bold: bool) -> f32 {
        let total_width: u32 = text.chars().map(Self::char_width_helvetica_bold).sum();
        // 1000単位をポイントに変換
        total_width as f32 / 1000.0 * font_size
    }

    /// センタリングされたX座標を計算（ページ中央基準）
    fn centered_x(text: &str, font_size: f32, is_bold: bool) -> f32 {
        let text_width = Self::estimate_text_width(text, font_size, is_bold);
        // A4横向きの中央（841.89pt / 2）からテキスト幅の半分を引く
        (A4_WIDTH_MM / 25.4 * 72.0 / 2.0) - (text_width / 2.0)
    }

    /// テキストのX座標を計算（centered設定に基づく）
    fn resolve_x(text: &str, font_size: f32, is_bold: bool, config_x: f32, centered: bool) -> f32 {
        if centered {
            Self::centered_x(text, font_size, is_bold)
        } else {
            config_x
        }
    }

    /// 白い縁取り付きでテキストを描画
    #[allow(clippy::too_many_arguments)]
    fn draw_text_with_outline(
        layer: &PdfLayerReference,
        text: &str,
        font_size: f32,
        x_mm: f32,
        y_mm: f32,
        fill_color: &Color,
        font: &IndirectFontRef,
        outline_width: f32,
    ) {
        let white = Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None));

        // 1. 白いストローク（縁取り）を描画
        layer.set_outline_color(white);
        layer.set_outline_thickness(outline_width);
        layer.set_text_rendering_mode(TextRenderingMode::Stroke);
        layer.use_text(text, font_size, Mm(x_mm), Mm(y_mm), font);

        // 2. 塗りつぶし（本体）を描画
        layer.set_fill_color(fill_color.clone());
        layer.set_text_rendering_mode(TextRenderingMode::Fill);
        layer.use_text(text, font_size, Mm(x_mm), Mm(y_mm), font);
    }

    fn add_text_overlay(
        &self,
        doc: &PdfDocumentReference,
        layer: &PdfLayerReference,
        info: &CertificateInfo,
        config: &TemplateConfig,
    ) -> Result<()> {
        // 組み込みフォントを使用（Helvetica Bold）
        let font = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

        // コールサインを描画（白縁取り付き）
        let cs = &config.callsign;
        let cs_color = Color::Rgb(Rgb::new(
            cs.color[0] as f32 / 255.0,
            cs.color[1] as f32 / 255.0,
            cs.color[2] as f32 / 255.0,
            None,
        ));

        let cs_x = Self::resolve_x(&info.callsign, cs.font_size, true, cs.x, cs.centered);

        // 縁取りの太さはフォントサイズの約10%
        let outline_width = cs.font_size * 0.10;
        Self::draw_text_with_outline(
            layer,
            &info.callsign,
            cs.font_size,
            cs_x * 25.4 / 72.0, // pt -> mm変換
            cs.y * 25.4 / 72.0,
            &cs_color,
            &font,
            outline_width,
        );

        // 達成内容を描画（白縁取り付き）
        let ach = &config.achievement;
        let ach_color = Color::Rgb(Rgb::new(
            ach.color[0] as f32 / 255.0,
            ach.color[1] as f32 / 255.0,
            ach.color[2] as f32 / 255.0,
            None,
        ));

        let ach_x = Self::resolve_x(
            &info.achievement_text,
            ach.font_size,
            true,
            ach.x,
            ach.centered,
        );

        let outline_width = ach.font_size * 0.10;
        Self::draw_text_with_outline(
            layer,
            &info.achievement_text,
            ach.font_size,
            ach_x * 25.4 / 72.0,
            ach.y * 25.4 / 72.0,
            &ach_color,
            &font,
            outline_width,
        );

        // 達成内容の2行目を描画（Bestなど）
        let mut next_y = ach.y - 40.0; // 達成内容の40pt下
        if let Some(ref line2) = info.achievement_line2 {
            let line2_font_size = ach.font_size * 0.8; // 達成内容より少し小さく
            let line2_x = Self::resolve_x(line2, line2_font_size, true, ach.x, ach.centered);
            let outline_width = line2_font_size * 0.10;

            Self::draw_text_with_outline(
                layer,
                line2,
                line2_font_size,
                line2_x * 25.4 / 72.0,
                next_y * 25.4 / 72.0,
                &ach_color,
                &font,
                outline_width,
            );
            next_y -= 30.0; // 次の行へ
        }

        // 説明文を描画（小さいフォント）
        if let Some(ref description) = info.description {
            let desc_font_size = 14.0; // 14pt
            let desc_x = Self::resolve_x(description, desc_font_size, true, ach.x, ach.centered);
            let outline_width = desc_font_size * 0.10;

            Self::draw_text_with_outline(
                layer,
                description,
                desc_font_size,
                desc_x * 25.4 / 72.0,
                next_y * 25.4 / 72.0,
                &ach_color,
                &font,
                outline_width,
            );
            next_y -= 24.0; // 次の行へ
        }

        // 発行日を描画（設定値を使用）
        if let Some(ref issue_date) = info.issue_date {
            let id = &config.issue_date;
            let id_color = Color::Rgb(Rgb::new(
                id.color[0] as f32 / 255.0,
                id.color[1] as f32 / 255.0,
                id.color[2] as f32 / 255.0,
                None,
            ));

            // Y座標: issue_date設定のyが指定されていればそれを使用、なければnext_yを使用
            let date_y = if id.y > 0.0 { id.y } else { next_y };
            let date_x = Self::resolve_x(issue_date, id.font_size, true, id.x, id.centered);
            let outline_width = id.font_size * 0.10;

            Self::draw_text_with_outline(
                layer,
                issue_date,
                id.font_size,
                date_x * 25.4 / 72.0,
                date_y * 25.4 / 72.0,
                &id_color,
                &font,
                outline_width,
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_award_type_template_filename() {
        assert_eq!(
            AwardType::Activator.template_filename(),
            "activator_template"
        );
        assert_eq!(AwardType::Chaser.template_filename(), "chaser_template");
    }

    #[test]
    fn test_template_not_exists() {
        let config = AwardTemplateConfig::default();
        let generator = AwardPdfGenerator::new("/nonexistent".to_string(), config);
        assert!(!generator.template_exists(AwardType::Activator));
        assert!(!generator.template_exists(AwardType::Chaser));
    }
}
