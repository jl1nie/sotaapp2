use aprs_message::AprsCallsign;
use chrono::{Duration, TimeZone, Utc};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::Write;

use super::admin_periodic::AdminPeriodicServiceImpl;
use super::user_service::UserServiceImpl;

use common::error::AppResult;
use common::utils::calculate_distance;
use domain::model::{
    activation::Spot,
    aprslog::{AprsLog, AprsState, AprsTrack},
    event::{FindActBuilder, FindAprs, FindRefBuilder},
};

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

    async fn send_message(
        &self,
        from: &AprsCallsign,
        message: &str,
        mesg_enabled: bool,
    ) -> AppResult<()> {
        tracing::info!(
            "APRS Message {}-{}({}): {}",
            from.callsign,
            from.ssid.unwrap_or_default(),
            mesg_enabled,
            message
        );
        if mesg_enabled {
            self.aprs_repo.write_message(from, message).await?;
        }
        Ok(())
    }

    pub async fn process_position(
        &self,
        from: AprsCallsign,
        latitude: f64,
        longitude: f64,
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
            .center(longitude, latitude, 3000.0)
            .build();

        let dest = self.sota_repo.find_reference(&query).await?;

        let time = Utc::now().naive_utc();

        if dest.is_empty() {
            let log = AprsLog {
                callsign: from,
                destination: None,
                state: AprsState::Travelling { time },
                longitude,
                latitude,
            };

            self.aprs_log_repo.insert_aprs_log(log).await?;

            return Ok(());
        }

        let summit = dest.first().unwrap().clone();
        let destination = summit.summit_code.clone();

        let patstr = self
            .config
            .aprs_arrival_mesg_regex
            .clone()
            .unwrap_or("$.".to_string());
        let patref = Regex::new(&patstr);

        let mesg_enabled = patref.is_ok_and(|r| r.is_match(&destination))
            && !self
                .config
                .aprs_exclude_user
                .as_ref()
                .is_some_and(|s| s.contains(&from.callsign));

        let (destlat, destlon) = (summit.latitude, summit.longitude);

        let query = FindAprs {
            callsign: Some(from.clone()),
            ..Default::default()
        };
        let aprslog = self.aprs_log_repo.find_aprs_log(&query).await?;

        let distance = calculate_distance(latitude, longitude, destlat, destlon).floor();

        let new_state = if distance > 1000.0 {
            AprsState::Approaching { time, distance }
        } else if distance > 300.0 {
            AprsState::Climbing { time, distance }
        } else if distance > 100.0 {
            let message = format!(
                "Approaching {}. {}m remaining.",
                summit.summit_code, distance
            );
            AprsState::NearSummit {
                time,
                distance,
                message,
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
                message,
            }
        };

        let state = match aprslog.first() {
            None => {
                if let Some(message) = new_state.message() {
                    self.send_message(&from, message, mesg_enabled).await?;
                }
                new_state
            }
            Some(prev_log) => {
                if prev_log.destination != Some(destination.clone()) {
                    new_state
                } else {
                    let old_state = prev_log.state.clone();
                    match (&old_state, &new_state) {
                        (AprsState::NearSummit { .. }, AprsState::OnSummit { .. })
                        | (
                            AprsState::Approaching { .. } | AprsState::Climbing { .. },
                            AprsState::NearSummit { .. },
                        )
                        | (
                            AprsState::Approaching { .. } | AprsState::Climbing { .. },
                            AprsState::OnSummit { .. },
                        ) => {
                            if let Some(mesg) = new_state.message() {
                                self.send_message(&from, mesg, mesg_enabled).await?;
                            }
                            new_state
                        }
                        (AprsState::OnSummit { .. }, AprsState::OnSummit { .. }) => new_state,
                        (AprsState::OnSummit { .. }, _) => AprsState::Descending { time, distance },
                        _ => old_state,
                    }
                }
            }
        };

        let log = AprsLog {
            callsign: from,
            destination: Some(destination),
            state,
            longitude,
            latitude,
        };

        self.aprs_log_repo.insert_aprs_log(log).await?;

        Ok(())
    }
}

impl UserServiceImpl {
    pub async fn generate_track(&self, aprslog: Vec<AprsLog>) -> AppResult<Vec<AprsTrack>> {
        let mut track: HashMap<AprsCallsign, Vec<(f64, f64)>> = HashMap::new();
        let mut lastlog: HashMap<AprsCallsign, AprsLog> = HashMap::new();

        for l in aprslog {
            let callsign = l.callsign.clone();

            track
                .entry(callsign.clone())
                .or_default()
                .push((l.latitude, l.longitude));

            lastlog.entry(callsign).or_insert(l);
        }

        let mut result = Vec::new();

        for callsign in track.keys() {
            let query = FindActBuilder::default()
                .sota()
                .operator(&callsign.callsign)
                .issued_after(Utc::now() - Duration::hours(8))
                .build();
            let spot = self.act_repo.find_spots(&query).await?;

            let log = lastlog.get(callsign).unwrap();
            let lastseen = Utc.from_utc_datetime(&log.state.time());

            let mut coordinates: Vec<_> = track.get(callsign).unwrap().to_vec();
            coordinates.reverse();

            let aprstrack = if let Some(spot) = spot.first() {
                AprsTrack {
                    callsign: callsign.clone(),
                    coordinates,
                    summit: Some(spot.reference.clone()),
                    distance: Some(log.state.distance()),
                    lastseen,
                    spot_time: Some(spot.spot_time),
                    spot_summit: Some(spot.reference.clone()),
                    spot_freq: Some(spot.frequency.clone()),
                    spot_mode: Some(spot.mode.clone()),
                    spot_comment: spot.comment.clone(),
                }
            } else {
                AprsTrack {
                    callsign: callsign.clone(),
                    coordinates,
                    summit: log.destination.clone(),
                    distance: Some(log.state.distance()),
                    lastseen,
                    spot_time: None,
                    spot_summit: None,
                    spot_freq: None,
                    spot_mode: None,
                    spot_comment: None,
                }
            };
            result.push(aprstrack);
        }
        Ok(result)
    }
}
