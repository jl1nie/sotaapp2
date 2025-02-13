use super::admin_periodic::AdminPeriodicServiceImpl;
use aprs_message::AprsCallsign;
use chrono::{Duration, Utc};
use common::error::AppResult;
use common::utils::calculate_distance;
use domain::model::{
    activation::Spot,
    aprslog::{AprsLog, AprsState},
    event::{FindActBuilder, FindRefBuilder},
};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::Write;

impl AdminPeriodicServiceImpl {
    async fn last_three_spots_messasge(&self, pat: &str) -> AppResult<String> {
        let after = Utc::now() - Duration::hours(3);
        let query = FindActBuilder::default().sota().issued_after(after).build();
        let mut spots = self.act_repo.find_spots(&query).await?;

        let pat = Regex::new(pat).unwrap();
        spots.retain(|r| pat.is_match(&r.reference));

        let mut latest: HashMap<String, Spot> = HashMap::new();
        for s in spots {
            latest
                .entry(s.activator.clone())
                .and_modify(|t| {
                    if s.spot_time > t.spot_time {
                        *t = s.clone();
                    }
                })
                .or_insert(s);
        }

        let mut spots: Vec<_> = latest.into_values().collect();
        spots.sort_by(|a, b| b.spot_time.cmp(&a.spot_time));
        spots.truncate(3);

        let mut message = String::new();

        if spots.is_empty() {
            message = "No Spots.".to_string();
        } else {
            for s in spots {
                write!(
                    &mut message,
                    "{}-{}-{} ",
                    s.spot_time.format("%H:%M"),
                    s.activator,
                    s.frequency
                )
                .unwrap();
            }
        }

        Ok(message)
    }

    pub async fn process_message(&self, from: &AprsCallsign, message: String) -> AppResult<()> {
        let message = message.to_uppercase();
        let commands: Vec<_> = message.split_ascii_whitespace().collect();

        if commands.is_empty() || commands.len() > 2 {
            self.aprs_repo.write_message(from, "?").await?;
            return Ok(());
        }

        let pat = match commands[0] {
            "DX" => r".*",
            "JA" => r"^JA.*",
            _ => r"^JA.*",
        };

        let message = self.last_three_spots_messasge(pat).await?;

        self.aprs_repo.write_message(from, &message).await?;

        Ok(())
    }

    pub async fn process_position(
        &self,
        from: &AprsCallsign,
        longitude: f64,
        latitude: f64,
    ) -> AppResult<()> {
        let query = FindActBuilder::default()
            .sota()
            .operator(&from.callsign)
            .build();
        let alert = self.act_repo.find_alerts(&query).await?;

        if alert.is_empty() {
            tracing::error!("Unknown activator {}:{:?}", &from.callsign, &alert);
            return Ok(());
        }

        let query = FindRefBuilder::default()
            .sota_code(alert[0].reference.clone())
            .build();
        let dest = self.sota_repo.find_reference(&query).await?;

        if dest.is_empty() {
            tracing::error!("Unknown destination {}", &alert[0].reference);
            return Ok(());
        }

        let summit = dest[0].clone();
        let destination = summit.summit_code.clone();

        let (destlon, destlat) = (
            summit.longitude.unwrap_or_default(),
            summit.latitude.unwrap_or_default(),
        );

        let aprslog = self.aprs_log_repo.get_aprs_log_by_callsign(from).await?;

        let time = Utc::now().naive_utc();
        let distance = calculate_distance(latitude, longitude, destlat, destlon).floor();

        let new_state = if distance > 1000.0 {
            AprsState::Approaching { time, distance }
        } else if distance > 300.0 {
            AprsState::Climbing { time, distance }
        } else if distance > 100.0 {
            AprsState::NearSummit {
                time,
                distance,
                message: Some(format!(
                    "Approaching {}. {}m to go.",
                    summit.summit_code, distance
                )),
            }
        } else {
            let message = if destination.starts_with("JA") {
                format!(
                    "Welcome to {}. {} {}m {}pts.\n{}\n{}",
                    summit.summit_code,
                    summit.summit_name,
                    summit.alt_m,
                    summit.points,
                    summit.city.unwrap_or_default(),
                    self.last_three_spots_messasge("^JA.*").await?
                )
            } else {
                format!(
                    "Welcome to {}. {} {}m {}pts.\n{}",
                    summit.summit_code,
                    summit.summit_name,
                    summit.alt_m,
                    summit.points,
                    self.last_three_spots_messasge(".*").await?
                )
            };

            AprsState::OnSummit {
                time,
                distance,
                message: Some(message),
            }
        };

        let old_state = aprslog.first();

        let state = if old_state.is_none() {
            match new_state {
                AprsState::NearSummit {
                    message: Some(ref message),
                    ..
                } => {
                    tracing::info!(
                        "APRS Message {}-{}: {}",
                        from.callsign,
                        from.ssid.unwrap_or_default(),
                        message
                    );
                    /*
                    if destination.starts_with("JA") {
                        self.aprs_repo.write_message(from, message).await?;
                    }
                    */
                }
                AprsState::OnSummit {
                    message: Some(ref message),
                    ..
                } => {
                    tracing::info!(
                        "APRS Message {}-{}: {}",
                        from.callsign,
                        from.ssid.unwrap_or_default(),
                        message
                    );
                    /*
                    if destination.starts_with("JA") {
                        self.aprs_repo.write_message(from, message).await?;
                    }
                    */
                }
                _ => {}
            }
            new_state
        } else {
            let old_state = old_state.unwrap().state.clone();
            match old_state {
                AprsState::NearSummit { .. } => match new_state {
                    AprsState::OnSummit {
                        message: Some(ref message),
                        ..
                    } => {
                        tracing::info!(
                            "APRS Message {}-{}: {}",
                            from.callsign,
                            from.ssid.unwrap_or_default(),
                            message
                        );
                        /*
                        if destination.starts_with("JA") {
                            self.aprs_repo.write_message(from, message).await?;
                        }
                        */
                        new_state
                    }
                    _ => old_state,
                },
                AprsState::Approaching { .. } | AprsState::Climbing { .. } => match new_state {
                    AprsState::NearSummit {
                        message: Some(ref message),
                        ..
                    } => {
                        tracing::info!(
                            "APRS Message {}-{}: {}",
                            from.callsign,
                            from.ssid.unwrap_or_default(),
                            message
                        );
                        /*
                        if destination.starts_with("JA") {
                            self.aprs_repo.write_message(from, message).await?;
                        }
                        */
                        new_state
                    }
                    AprsState::OnSummit {
                        message: Some(ref message),
                        ..
                    } => {
                        tracing::info!(
                            "APRS Message {}-{}: {}",
                            from.callsign,
                            from.ssid.unwrap_or_default(),
                            message
                        );
                        /*
                        if destination.starts_with("JA") {
                            self.aprs_repo.write_message(from, message).await?;
                        }
                        */
                        new_state
                    }
                    _ => new_state,
                },
                AprsState::OnSummit { .. } => match new_state {
                    AprsState::OnSummit { .. } => new_state,
                    _ => AprsState::Descending { time, distance },
                },
                AprsState::Descending { .. } => old_state,
            }
        };

        let log = AprsLog {
            callsign: AprsCallsign {
                callsign: from.callsign.clone(),
                ssid: from.ssid,
            },
            destination,
            state,
            longitude,
            latitude,
        };

        tracing::info!("APRS Beacon:{:?}", log);

        self.aprs_log_repo.insert_aprs_log(log).await?;

        Ok(())
    }
}
