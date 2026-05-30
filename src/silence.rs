//! Silence detection: tracks whether the subject has been quiet all day.
//!
//! Emits at most one `wm.presence.silence` per waking-hours window.
//! Outside the window, silence is expected and never reported.

use chrono::{DateTime, Local, NaiveTime, Timelike as _, Utc};

use crate::config::PresenceConfig;
use crate::state::DailyState;

/// The result of a silence check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SilenceResult {
    /// Should emit `wm.presence.silence` now.
    EmitSilence,
    /// Silence already emitted for this window — debounce.
    AlreadyEmitted,
    /// There have been interactions today — no silence.
    HasInteractions,
    /// The current time is outside the waking-hours window.
    OutsideWindow,
    /// The silence threshold time has not yet been reached.
    BeforeThreshold,
}

/// Stateless silence detector.
pub struct SilenceDetector<'a> {
    cfg: &'a PresenceConfig,
}

impl<'a> SilenceDetector<'a> {
    /// Create a new detector for the given config.
    #[must_use]
    pub const fn new(cfg: &'a PresenceConfig) -> Self {
        Self { cfg }
    }

    /// Check whether silence should be emitted at `now` (UTC) given `state`.
    ///
    /// Converts `now` to local time for comparison against the configured
    /// waking-hours window times.
    #[must_use]
    pub fn check(&self, now: DateTime<Utc>, state: &DailyState) -> SilenceResult {
        let local_now = now.with_timezone(&Local);
        self.check_local(local_now.time(), state)
    }

    /// Check using a local `NaiveTime` directly (useful for deterministic tests).
    #[must_use]
    pub fn check_local(&self, local_time: NaiveTime, state: &DailyState) -> SilenceResult {
        // 1. Outside waking hours? (before start OR at/after end)
        if local_time < self.cfg.waking_start || local_time >= self.cfg.waking_end {
            return SilenceResult::OutsideWindow;
        }

        // 2. Before the silence threshold?
        if local_time < self.cfg.silence_threshold {
            return SilenceResult::BeforeThreshold;
        }

        // 3. Already emitted for this window?
        if state.silence_emitted_for_window {
            return SilenceResult::AlreadyEmitted;
        }

        // 4. Any interactions today?
        if state.daily_count > 0 {
            return SilenceResult::HasInteractions;
        }

        SilenceResult::EmitSilence
    }

    /// Format a human-readable window description for the silence payload.
    #[must_use]
    pub fn window_description(&self) -> String {
        format!(
            "{:02}:{:02}–{:02}:{:02} local",
            self.cfg.waking_start.hour(),
            self.cfg.waking_start.minute(),
            self.cfg.waking_end.hour(),
            self.cfg.waking_end.minute(),
        )
    }
}
