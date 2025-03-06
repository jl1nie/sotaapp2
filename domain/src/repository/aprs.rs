use aprs_message::{AprsCallsign, AprsData};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use common::error::AppResult;
use shaku::Interface;

use crate::model::aprslog::AprsLog;
use crate::model::event::FindAprs;

#[async_trait]
pub trait AprsRepositry: Send + Sync + Interface {
    async fn write_message(&self, addressee: &AprsCallsign, message: &str) -> AppResult<()>;
    async fn set_buddy_list(&self, buddy: Vec<String>) -> AppResult<()>;
    async fn set_filter(&self, filter: String) -> AppResult<()>;
    async fn get_aprs_packet(&self) -> AppResult<AprsData>;
}
#[async_trait]
pub trait AprsLogRepository: Send + Sync + Interface {
    async fn find_aprs_log(&self, query: &FindAprs) -> AppResult<Vec<AprsLog>>;
    async fn insert_aprs_log(&self, aprs_log: AprsLog) -> AppResult<()>;
    async fn delete_aprs_log(&self, before: &NaiveDateTime) -> AppResult<()>;
}
