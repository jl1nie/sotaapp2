use anyhow::Result;
use chrono::NaiveDate;
use reqwest;
use shaku::HasComponent;
use std::sync::Arc;

use common::config::AppConfig;
use domain::model::geomag::GeomagIndex;
use registry::{AppRegistry, AppState};
use service::services::AdminPeriodicService;

#[derive(Clone)]
pub struct UpdateGeoMag {
    config: AppConfig,
    registry: Arc<AppRegistry>,
}

impl UpdateGeoMag {
    pub fn new(config: &AppConfig, state: &AppState) -> Self {
        Self {
            config: config.clone(),
            registry: state.into(),
        }
    }

    pub async fn update(&self) -> Result<()> {
        let service: &dyn AdminPeriodicService = self.registry.resolve_ref();
        let endpoint = self.config.geomag_endpoint.clone();
        let response = reqwest::get(&endpoint).await?.text().await?;
        let lines: Vec<_> = response.lines().rev().take(2).collect();

        let date: Vec<NaiveDate> = lines
            .iter()
            .filter_map(|line| {
                line.get(0..10)
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y %m %d").ok())
            })
            .collect();

        let ap: Vec<i32> = lines
            .iter()
            .filter_map(|line| line.get(60..62).and_then(|s| s.parse().ok()))
            .collect();

        let kp = lines
            .iter()
            .map(|line| {
                line.get(63..)
                    .map(|part| {
                        part.split_whitespace()
                            .filter_map(|s| s.parse::<f32>().ok())
                            .rev()
                            .filter(|&k| k > 0.0f32)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(Vec::new)
            })
            .collect::<Vec<_>>();

        let mut index = GeomagIndex::default();

        if ap[0] < 0 {
            index.date = date[1];
            index.a_index = ap[1];
            index.k_index = kp[1][0]
        } else {
            index.date = date[0];
            index.a_index = ap[0];
            index.k_index = kp[0][0];
        }

        service.update_geomag(index).await?;
        Ok(())
    }
}
