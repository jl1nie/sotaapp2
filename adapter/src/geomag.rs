use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use common::config::AppConfig;
use common::error::{AppError, AppResult};
use domain::{model::geomag::GeomagIndex, repository::geomag::GeoMagRepositry};
use shaku::Component;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn connect_geomag_with(cfg: &AppConfig) -> Result<GeoMag> {
    let geomag = GeoMag::new(cfg).await?;
    Ok(geomag)
}

#[derive(Clone)]
pub struct GeoMag {
    geomag: Arc<Mutex<Option<GeomagIndex>>>,
}

impl GeoMag {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let endpoint = config.geomag_endpoint.clone();
        let schedule = config.geomag_update_schedule.clone();

        let geomag = Arc::new(Mutex::new(Some(GeomagIndex::default())));
        let geomag_clone = geomag.clone();

        if let Err(e) = Self::update(endpoint.as_str(), geomag.clone()).await {
            tracing::error!("Geomag update error: {}", e);
        }

        let sched = JobScheduler::new().await?;
        sched
            .add(
                Job::new_async(&schedule, move |_uuid, _l| {
                    let endpoint = endpoint.clone();
                    let geomag = geomag_clone.clone();
                    Box::pin(async move {
                        if let Err(e) = Self::update(endpoint.as_str(), geomag.clone()).await {
                            tracing::error!("Geomag update error: {}", e);
                        }
                    })
                })
                .unwrap_or_else(|_| panic!("Bad cron format: {}", &schedule)),
            )
            .await?;

        sched.start().await?;

        Ok(Self { geomag })
    }

    pub async fn get_geomag(&self) -> AppResult<Option<GeomagIndex>> {
        let geomag = self.geomag.lock().await;
        Ok(geomag.clone())
    }

    async fn update(endpoint: &str, index: Arc<Mutex<Option<GeomagIndex>>>) -> AppResult<()> {
        let response = reqwest::get(endpoint)
            .await
            .map_err(AppError::GetError)?
            .text()
            .await
            .map_err(AppError::GetError)?;

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
            .filter_map(|line| line.get(60..62).and_then(|s| s.trim().parse().ok()))
            .collect();

        let kp = lines
            .iter()
            .map(|line| {
                line.get(63..)
                    .map(|part| {
                        part.split_whitespace()
                            .filter_map(|s| s.trim().parse::<f32>().ok())
                            .rev()
                            .filter(|&k| k > 0.0f32)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(Vec::new)
            })
            .collect::<Vec<_>>();

        let mut new_index = GeomagIndex::default();
        if ap.len() > 1 && kp.len() > 1 {
            if ap[0] < 0 {
                new_index.date = date[1];
                new_index.a_index = ap[1];
                new_index.k_index = kp.get(1).cloned().unwrap_or(vec![]);
            } else {
                new_index.date = date[0];
                new_index.a_index = ap[0];
                new_index.k_index = kp.first().cloned().unwrap_or(vec![]);
            }
            let mut index = index.lock().await;
            tracing::info!("Update GeomagIndex {:?}", &new_index);

            *index = Some(new_index);

            Ok(())
        } else {
            Err(AppError::UnprocessableEntity(format!(
                "GeoMag file fortmat error: {}",
                endpoint
            )))
        }
    }
}

#[derive(Component)]
#[shaku(interface = GeoMagRepositry)]
pub struct GeoMagRepositryImpl {
    geomag: GeoMag,
}

#[async_trait]
impl GeoMagRepositry for GeoMagRepositryImpl {
    async fn get_geomag(&self) -> AppResult<Option<GeomagIndex>> {
        let latest_data = self.geomag.get_geomag().await?;
        Ok(latest_data.clone())
    }
}
