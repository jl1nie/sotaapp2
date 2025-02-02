use aprs_message::AprsData;
use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

#[async_trait]
pub trait AprsRepositry: Send + Sync + Interface {
    async fn write_message(&self, addressee: &str, message: &str) -> AppResult<()>;
    async fn set_buddy_list(&self, buddy: Vec<String>) -> AppResult<()>;
    async fn get_aprs_packet(&self) -> AppResult<AprsData>;
}
