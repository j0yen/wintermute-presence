//! Configuration for the presence daemon.
//!
//! Reads from `/etc/wintermute/conf.d/presence.toml` (or a test-supplied path).
//! If the file is absent or `enabled = false`, the daemon is a no-op.

use anyhow::{Context as _, Result};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Time-of-day alias for clarity.
pub type TimeOfDay = NaiveTime;

/// Top-level presence configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PresenceConfig {
    /// Opt-in gate. If false, the daemon subscribes nothing and emits nothing.
    pub enabled: bool,

    /// Start of the subject's waking hours (local time, HH:MM).
    #[serde(with = "naive_time_serde")]
    pub waking_start: TimeOfDay,

    /// End of the subject's waking hours (local time, HH:MM).
    #[serde(with = "naive_time_serde")]
    pub waking_end: TimeOfDay,

    /// Time at which silence is declared if no interaction has occurred
    /// since `waking_start` (local time, HH:MM). Must be within the window.
    #[serde(with = "naive_time_serde")]
    pub silence_threshold: TimeOfDay,
}

/// Build a `NaiveTime` from known-valid hour/minute/second constants.
///
/// Returns `NaiveTime::MIN` (00:00:00) only when called with out-of-range
/// values, which never happens for the hardcoded defaults below.
fn default_time(h: u32, m: u32, s: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, s).unwrap_or(NaiveTime::MIN)
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            waking_start: default_time(8, 0, 0),
            waking_end: default_time(21, 0, 0),
            silence_threshold: default_time(12, 0, 0),
        }
    }
}

/// Default config directory path.
#[must_use]
pub fn default_conf_dir() -> PathBuf {
    PathBuf::from("/etc/wintermute/conf.d")
}

/// Load presence config from `<conf_dir>/presence.toml`.
///
/// Returns a disabled config (no-op) if the file is absent.
///
/// # Errors
///
/// Returns `Err` if the file exists but cannot be parsed.
pub fn load(conf_dir: &Path) -> Result<PresenceConfig> {
    let path = conf_dir.join("presence.toml");
    if !path.exists() {
        return Ok(PresenceConfig::default());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

mod naive_time_serde {
    use chrono::NaiveTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FMT: &str = "%H:%M";

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(super) fn serialize<S>(t: &NaiveTime, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&t.format(FMT).to_string())
    }

    pub(super) fn deserialize<'de, D>(d: D) -> Result<NaiveTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        NaiveTime::parse_from_str(&s, FMT).map_err(serde::de::Error::custom)
    }
}
