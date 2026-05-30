//! AC6 (MUST): Outside waking hours, no `wm.presence.silence` is ever emitted.
//! Tested with a local clock at 03:00.

use wintermute_presence::silence::{SilenceDetector, SilenceResult};
use wintermute_presence::config::PresenceConfig;
use wintermute_presence::state::DailyState;
use chrono::{NaiveDate, NaiveTime};

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap_or(NaiveTime::MIN)
}

#[test]
fn test_no_silence_outside_window() {
    let cfg = PresenceConfig {
        enabled: true,
        waking_start: t(8, 0),
        waking_end: t(21, 0),
        silence_threshold: t(12, 0),
    };

    let today = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap_or_default();

    // State: zero interactions, silence not emitted.
    let state = DailyState {
        date: today,
        daily_count: 0,
        last_interaction_ts: None,
        silence_emitted_for_window: false,
    };

    // Local time at 03:00 — outside waking hours.
    let local_time = t(3, 0);

    let result = SilenceDetector::new(&cfg).check_local(local_time, &state);
    assert_ne!(
        result,
        SilenceResult::EmitSilence,
        "silence must never be emitted outside waking hours (03:00)"
    );
    assert_eq!(
        result,
        SilenceResult::OutsideWindow,
        "result at 03:00 must be OutsideWindow"
    );
}
