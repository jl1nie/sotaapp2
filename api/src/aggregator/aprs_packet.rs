use common::error::AppResult;
use domain::repository::aprs::AprsRepositry;
use registry::AppRegistry;
use service::services::AdminPeriodicService;
use shaku::HasComponent;
use std::sync::Arc;

pub async fn process_incoming_packet(registry: &Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminPeriodicService = registry.resolve_ref();
    let aprs_repo: &dyn AprsRepositry = registry.resolve_ref();

    let packet = aprs_repo.get_aprs_packet().await?;
    service.aprs_packet_received(packet).await?;

    Ok(())
}
