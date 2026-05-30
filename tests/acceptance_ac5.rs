//! AC5 (MUST): An interaction inside the window suppresses silence emission.

use wintermute_presence::silence::{SilenceDetector, SilenceResult};
use wintermute_presence::config::PresenceConfig;
use wintermute_presence::state::DailyState;
use chrono::{NaiveDate, NaiveTime};

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap_or(NaiveTime::MIN)
}

#[test]
fn test_interaction_suppresses_silence() {
    let cfg = PresenceConfig {
        enabled: true,
        waking_start: t(8, 0),
        waking_end: t(21, 0),
        silence_threshold: t(12, 0),
    };

    let today = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap_or_default();

    // State: ONE interaction today.
    let state = DailyState {
        date: today,
        daily_count: 1,
        last_interaction_ts: Some(chrono::Utc::now()),
        silence_emitted_for_window: false,
    };

    // Local time at 12:01 — past threshold.
    let local_time = t(12, 1);

    let result = SilenceDetector::new(&cfg).check_local(local_time, &state);
    assert_eq!(
        result,
        SilenceResult::HasInteractions,
        "silence must be suppressed when interaction exists"
    );
}
