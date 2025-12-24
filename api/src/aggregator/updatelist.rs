use futures_util::StreamExt;
use shaku::HasComponent;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use common::error::AppResult;
use common::http;
use common::{config::AppConfig, error::AppError};
use registry::AppRegistry;
use service::services::AdminService;

const SOTA_CSV_PATH: &str = "/tmp/summits.csv";
const POTA_CSV_PATH: &str = "/tmp/parks.csv";

/// HTTPレスポンスをストリームでファイルに保存（メモリ効率向上）
async fn download_to_file(url: &str, path: &str, user_agent: Option<&str>) -> AppResult<()> {
    let client = http::client();
    let mut request = client.get(url);

    if let Some(ua) = user_agent {
        request = request.header(http::header::USER_AGENT, ua);
    }

    let response = request.send().await.map_err(AppError::GetError)?;

    if !response.status().is_success() {
        return Err(AppError::GetError(response.error_for_status().unwrap_err()));
    }

    let mut file = File::create(path)
        .await
        .map_err(|e| AppError::IoError(format!("Failed to create file {}: {}", path, e)))?;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(AppError::GetError)?;
        file.write_all(&chunk)
            .await
            .map_err(|e| AppError::IoError(format!("Failed to write to file: {}", e)))?;
    }

    file.flush()
        .await
        .map_err(|e| AppError::IoError(format!("Failed to flush file: {}", e)))?;

    Ok(())
}

pub async fn update_summit_list(config: AppConfig, registry: Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminService = registry.resolve_ref();
    let endpoint = config.sota_summitlist_endpoint.clone();

    tracing::info!("Downloading summit list from {}", endpoint);
    download_to_file(&endpoint, SOTA_CSV_PATH, None).await?;
    tracing::info!("Downloaded summit list to {}", SOTA_CSV_PATH);

    let count = service
        .update_summit_list_from_file(Path::new(SOTA_CSV_PATH))
        .await?;

    // 一時ファイル削除
    let _ = tokio::fs::remove_file(SOTA_CSV_PATH).await;

    if config.reboot_after_update {
        tokio::time::sleep(Duration::from_secs(10)).await;
        tracing::info!("Sending graceful shutdown signal.");
        let _ = config.shutdown_tx.send(true);
    }
    tracing::info!(
        "Summit list updated successfully. {} summits updated.",
        count
    );
    Ok(())
}

pub async fn update_park_list(config: AppConfig, registry: Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminService = registry.resolve_ref();
    let endpoint = config.pota_parklist_endpoint.clone();

    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";

    tracing::info!("Downloading park list from {}", endpoint);
    download_to_file(&endpoint, POTA_CSV_PATH, Some(user_agent)).await?;
    tracing::info!("Downloaded park list to {}", POTA_CSV_PATH);

    let count = service
        .update_pota_park_list_from_file(Path::new(POTA_CSV_PATH))
        .await?;

    // 一時ファイル削除
    let _ = tokio::fs::remove_file(POTA_CSV_PATH).await;

    tracing::info!("Park list updated successfully. {} parks updated.", count);
    Ok(())
}
