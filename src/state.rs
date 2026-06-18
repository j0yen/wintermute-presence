//! Daily interaction state — persisted to a JSON file for restart resilience.
//!
//! Writes atomically via `.tmp` + rename so a mid-write crash never corrupts.

use anyhow::{Context as _, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

use crate::hearing::HearingLiveness;

/// Per-day presence state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DailyState {
    /// Calendar date this record covers (local date).
    pub date: NaiveDate,
    /// Number of interactions (wm.audio.wake or wm.stt.final) today.
    pub daily_count: u64,
    /// UTC timestamp of the most recent interaction, if any.
    pub last_interaction_ts: Option<DateTime<Utc>>,
    /// True if we already emitted `wm.presence.silence` for today's window.
    pub silence_emitted_for_window: bool,
    /// True once a `wm.health.hearing` `ok` envelope arrived within today's
    /// waking-hours window.  Consumed by pulse-silence-gate to distinguish
    /// "quiet but confirmed hearing" from "deaf all day".
    #[serde(default)]
    pub hearing_confirmed_in_window: bool,
    /// Current hearing-liveness verdict, persisted across daemon restarts.
    ///
    /// Defaults to `Hearing` on first load (optimistic assumption until
    /// the first probe arrives).
    #[serde(default)]
    pub hearing_liveness: HearingLiveness,
    /// UTC timestamp of the last `ok` hearing probe, if any.
    #[serde(default)]
    pub hearing_last_ok_ts: Option<DateTime<Utc>>,
}

impl Default for DailyState {
    fn default() -> Self {
        Self::fresh(chrono::Local::now().date_naive())
    }
}

impl DailyState {
    /// Create a fresh state for `date` with zero interactions.
    #[must_use]
    pub fn fresh(date: NaiveDate) -> Self {
        Self {
            date,
            daily_count: 0,
            last_interaction_ts: None,
            silence_emitted_for_window: false,
            hearing_confirmed_in_window: false,
            hearing_liveness: HearingLiveness::default(),
            hearing_last_ok_ts: None,
        }
    }

    /// Mark that a confirmed-hearing probe landed in this window.
    #[must_use]
    pub fn with_hearing_confirmed(self) -> Self {
        Self {
            hearing_confirmed_in_window: true,
            ..self
        }
    }

    /// Update the persisted hearing liveness state and last-ok timestamp.
    #[must_use]
    pub fn with_hearing_liveness(
        self,
        liveness: HearingLiveness,
        last_ok_ts: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            hearing_liveness: liveness,
            hearing_last_ok_ts: last_ok_ts.or(self.hearing_last_ok_ts),
            ..self
        }
    }

    /// Record an interaction at `ts`, returning the updated state.
    #[must_use]
    pub fn with_interaction(self, ts: DateTime<Utc>) -> Self {
        Self {
            daily_count: self.daily_count.saturating_add(1),
            last_interaction_ts: Some(ts),
            ..self
        }
    }

    /// Mark silence as emitted for this window, returning the updated state.
    #[must_use]
    pub fn with_silence_emitted(self) -> Self {
        Self {
            silence_emitted_for_window: true,
            ..self
        }
    }
}

/// Handle to the state file on disk.
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    /// Create a store pointing at `path`.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Default state file path (`~/.local/state/wintermute-presence/state.json`).
    ///
    /// # Errors
    ///
    /// Returns `Err` if `$HOME` is not set.
    pub fn default_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("$HOME not set")?;
        Ok(PathBuf::from(home)
            .join(".local/state/wintermute-presence/state.json"))
    }

    /// Load state from disk.
    ///
    /// Returns a fresh state for today if the file is absent or for a prior day.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the file exists but cannot be parsed.
    pub async fn load(&self) -> Result<DailyState> {
        let today = chrono::Local::now().date_naive();
        if !self.path.exists() {
            return Ok(DailyState::fresh(today));
        }
        let raw = fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("reading state file {}", self.path.display()))?;
        let state: DailyState = serde_json::from_str(&raw)
            .with_context(|| format!("parsing state file {}", self.path.display()))?;
        // If the stored date is not today, reset.
        if state.date != today {
            return Ok(DailyState::fresh(today));
        }
        Ok(state)
    }

    /// Atomically write state to disk.
    ///
    /// # Errors
    ///
    /// Returns `Err` on serialization or I/O failure.
    pub async fn save(&self, state: &DailyState) -> Result<()> {
        // Ensure parent directory exists.
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating state dir {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(state).context("serializing state")?;
        let tmp = self.path.with_extension("tmp");
        fs::write(&tmp, json.as_bytes())
            .await
            .with_context(|| format!("writing tmp state file {}", tmp.display()))?;
        fs::rename(&tmp, &self.path)
            .await
            .with_context(|| {
                format!(
                    "renaming {} -> {}",
                    tmp.display(),
                    self.path.display()
                )
            })?;
        Ok(())
    }
}
