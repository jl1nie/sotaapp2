use serde::Serialize;

/// Import operation result
#[derive(Debug, Clone, Serialize, Default)]
pub struct ImportResult {
    /// Whether the import was successful
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Number of records successfully imported
    pub imported: u32,
    /// Number of records skipped (duplicates, etc.)
    pub skipped: u32,
    /// List of errors encountered during import
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ImportError>,
}

/// Individual import error
#[derive(Debug, Clone, Serialize)]
pub struct ImportError {
    /// Line number where the error occurred (1-indexed)
    pub line: u32,
    /// Error message
    pub message: String,
}

impl ImportResult {
    /// Create a successful result with counts
    pub fn success(imported: u32, skipped: u32) -> Self {
        Self {
            success: true,
            message: format!("インポート完了: {}件追加, {}件スキップ", imported, skipped),
            imported,
            skipped,
            errors: vec![],
        }
    }

    /// Create a failed result with error message
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            imported: 0,
            skipped: 0,
            errors: vec![],
        }
    }

    /// Create a partial success result with errors
    pub fn partial(imported: u32, skipped: u32, errors: Vec<ImportError>) -> Self {
        Self {
            success: true,
            message: format!(
                "インポート完了: {}件追加, {}件スキップ, {}件エラー",
                imported,
                skipped,
                errors.len()
            ),
            imported,
            skipped,
            errors,
        }
    }
}

impl ImportError {
    pub fn new(line: u32, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}
