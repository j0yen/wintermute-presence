//! AC4 (MUST): With zero interactions inside the configured waking-hours window,
//! exactly one `wm.presence.silence` is emitted after the threshold.
//! Subsequent ticks emit none (debounce).

use wintermute_presence::silence::{SilenceDetector, SilenceResult};
use wintermute_presence::config::PresenceConfig;
use wintermute_presence::state::DailyState;
use chrono::{NaiveDate, NaiveTime};

/// Build a NaiveTime from h:m.
fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap_or(NaiveTime::MIN)
}

#[test]
fn test_silence_debounce() {
    let cfg = PresenceConfig {
        enabled: true,
        waking_start: t(8, 0),
        waking_end: t(21, 0),
        silence_threshold: t(12, 0),
    };

    let today = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap_or_default();

    // State: zero interactions, silence not yet emitted.
    let mut state = DailyState {
        date: today,
        daily_count: 0,
        last_interaction_ts: None,
        silence_emitted_for_window: false,
    };

    // Local time at 12:01 — past threshold, no interactions.
    let local_time = t(12, 1);

    let detector = SilenceDetector::new(&cfg);

    // First tick: should emit.
    let result = detector.check_local(local_time, &state);
    assert_eq!(result, SilenceResult::EmitSilence, "first tick must emit silence");

    // Mark silence as emitted.
    state.silence_emitted_for_window = true;

    // Second tick: debounced — must NOT emit.
    let result2 = detector.check_local(local_time, &state);
    assert_eq!(result2, SilenceResult::AlreadyEmitted, "second tick must be debounced");
}
