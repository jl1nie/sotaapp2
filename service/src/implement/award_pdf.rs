//! アワード証明書PDF生成

use anyhow::{Context, Result};
use common::award_config::{AwardTemplateConfig, TemplateConfig};
use lopdf::{Document, Object, ObjectId, Stream};
use std::path::Path;

/// アワードの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwardType {
    Activator,
    Chaser,
}

impl AwardType {
    pub fn template_filename(&self) -> &'static str {
        match self {
            AwardType::Activator => "activator_template.pdf",
            AwardType::Chaser => "chaser_template.pdf",
        }
    }
}

/// アワード証明書の生成情報
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub callsign: String,
    pub achievement_text: String,
}

/// アワードPDF生成器
pub struct AwardPdfGenerator {
    template_dir: String,
    config: AwardTemplateConfig,
}

impl AwardPdfGenerator {
    pub fn new(template_dir: String, config: AwardTemplateConfig) -> Self {
        Self { template_dir, config }
    }

    /// テンプレートが存在するかチェック
    pub fn template_exists(&self, award_type: AwardType) -> bool {
        let path = Path::new(&self.template_dir).join(award_type.template_filename());
        path.exists()
    }

    /// PDF証明書を生成
    pub fn generate(&self, award_type: AwardType, info: &CertificateInfo) -> Result<Vec<u8>> {
        let template_path = Path::new(&self.template_dir).join(award_type.template_filename());

        if !template_path.exists() {
            anyhow::bail!("テンプレートファイルが存在しません: {:?}", template_path);
        }

        let mut doc = Document::load(&template_path)
            .with_context(|| format!("PDFテンプレートの読み込みに失敗: {:?}", template_path))?;

        let template_config = match award_type {
            AwardType::Activator => &self.config.activator,
            AwardType::Chaser => &self.config.chaser,
        };

        // テキストをオーバーレイ
        self.add_text_overlay(&mut doc, info, template_config)?;

        // PDFをバイト列として出力
        let mut buffer = Vec::new();
        doc.save_to(&mut buffer)
            .context("PDFの保存に失敗")?;

        Ok(buffer)
    }

    fn add_text_overlay(
        &self,
        doc: &mut Document,
        info: &CertificateInfo,
        config: &TemplateConfig,
    ) -> Result<()> {
        // ページを取得（page_number -> ObjectId のマップ）
        let pages = doc.get_pages();
        let first_page_id: ObjectId = *pages
            .values()
            .next()
            .context("PDFにページがありません")?;

        // コンテンツストリームを作成
        let content = self.create_text_content_stream(info, config);

        // 新しいストリームオブジェクトを追加
        let stream = Stream::new(Default::default(), content.into_bytes());
        let stream_id = doc.add_object(Object::Stream(stream));

        // ページのContentsに追加
        let page = doc.get_object_mut(first_page_id)
            .context("ページオブジェクトの取得に失敗")?;

        if let Object::Dictionary(ref mut dict) = page {
            // 既存のContentsを取得（Result型なのでOkの場合のみ処理）
            let existing_contents = dict.get(b"Contents").ok().cloned();

            // 新しいContents配列を作成
            let new_contents = match existing_contents {
                Some(Object::Array(mut arr)) => {
                    arr.push(Object::Reference(stream_id));
                    Object::Array(arr)
                }
                Some(Object::Reference(ref_id)) => {
                    Object::Array(vec![Object::Reference(ref_id), Object::Reference(stream_id)])
                }
                _ => Object::Reference(stream_id),
            };

            dict.set(b"Contents", new_contents);
        }

        Ok(())
    }

    fn create_text_content_stream(&self, info: &CertificateInfo, config: &TemplateConfig) -> String {
        let mut content = String::new();

        // グラフィックス状態を保存
        content.push_str("q\n");

        // コールサインの描画
        let cs = &config.callsign;
        content.push_str(&format!(
            "BT\n/{} {} Tf\n{} {} {} rg\n{} {} Td\n({}) Tj\nET\n",
            "Helvetica-Bold",
            cs.font_size,
            cs.color[0] as f32 / 255.0,
            cs.color[1] as f32 / 255.0,
            cs.color[2] as f32 / 255.0,
            cs.x,
            cs.y,
            Self::escape_pdf_string(&info.callsign)
        ));

        // 達成内容の描画
        let ach = &config.achievement;
        content.push_str(&format!(
            "BT\n/{} {} Tf\n{} {} {} rg\n{} {} Td\n({}) Tj\nET\n",
            "Helvetica",
            ach.font_size,
            ach.color[0] as f32 / 255.0,
            ach.color[1] as f32 / 255.0,
            ach.color[2] as f32 / 255.0,
            ach.x,
            ach.y,
            Self::escape_pdf_string(&info.achievement_text)
        ));

        // グラフィックス状態を復元
        content.push_str("Q\n");

        content
    }

    fn escape_pdf_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::award_config::TextOverlayConfig;

    #[test]
    fn test_award_type_template_filename() {
        assert_eq!(AwardType::Activator.template_filename(), "activator_template.pdf");
        assert_eq!(AwardType::Chaser.template_filename(), "chaser_template.pdf");
    }

    #[test]
    fn test_escape_pdf_string() {
        assert_eq!(AwardPdfGenerator::escape_pdf_string("JA1XXX"), "JA1XXX");
        assert_eq!(AwardPdfGenerator::escape_pdf_string("test(1)"), "test\\(1\\)");
        assert_eq!(AwardPdfGenerator::escape_pdf_string("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_create_text_content_stream() {
        let config = AwardTemplateConfig::default();
        let generator = AwardPdfGenerator::new("./templates".to_string(), config.clone());

        let info = CertificateInfo {
            callsign: "JA1XXX".to_string(),
            achievement_text: "10 summits activated".to_string(),
        };

        let content = generator.create_text_content_stream(&info, &config.activator);

        assert!(content.contains("JA1XXX"));
        assert!(content.contains("10 summits activated"));
        assert!(content.starts_with("q\n"));
        assert!(content.ends_with("Q\n"));
    }
}
