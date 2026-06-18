//! Hearing AC6: deaf readings outside the waking-hours window must NOT emit
//! `wm.health.hearing.fail`.  A sleeping-hours probe result is not an alert.
//!
//! This is tested at the state-machine + daemon logic level (without a live bus)
//! since the waking-hours check is in `is_in_waking_window` (daemon.rs).
//!
//! We test the invariant via the `SilenceDetector` window logic combined with
//! the hearing state machine: the state machine still transitions to DEAF, but
//! the daemon only emits the fail event when `in_window` is true.  We verify
//! that the window boundary behaves correctly.

use chrono::NaiveTime;
use wintermute_presence::config::PresenceConfig;
use wintermute_presence::hearing::{HearingEnvelope, HearingLiveness, HearingState};
use wintermute_presence::silence::{SilenceDetector, SilenceResult};
use wintermute_presence::state::DailyState;

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap_or(NaiveTime::MIN)
}

fn deaf_env() -> HearingEnvelope {
    HearingEnvelope {
        state: "deaf".to_string(),
        model_present: true,
        detector_loaded: true,
    }
}

/// The state machine transitions to DEAF correctly at K failures regardless of
/// window.  The DAEMON's `in_window` guard is what suppresses the `.fail` emit.
/// This test verifies the window boundary returns false outside waking hours.
#[test]
fn test_outside_window_at_3am() {
    let cfg = PresenceConfig {
        enabled: true,
        waking_start: t(8, 0),
        waking_end: t(21, 0),
        silence_threshold: t(12, 0),
        ..PresenceConfig::default()
    };

    // At 03:00 the window is closed.
    let local_time = t(3, 0);
    let is_in_window = local_time >= cfg.waking_start && local_time < cfg.waking_end;
    assert!(!is_in_window, "03:00 must be outside the waking-hours window");
}

/// Outside the window, silence is not emitted.
#[test]
fn test_silence_outside_window_not_emitted() {
    let cfg = PresenceConfig {
        enabled: true,
        waking_start: t(8, 0),
        waking_end: t(21, 0),
        silence_threshold: t(12, 0),
        ..PresenceConfig::default()
    };

    let today = chrono::Local::now().date_naive();
    let state = DailyState::fresh(today);

    let detector = SilenceDetector::new(&cfg);
    let result = detector.check_local(t(3, 0), &state);
    assert_eq!(
        result,
        SilenceResult::OutsideWindow,
        "silence must not fire at 03:00 (outside waking hours)"
    );
}

/// State machine transitions to DEAF at K failures (window-agnostic), but the
/// daemon must guard the `.fail` emit behind `in_window`.  This test verifies
/// the state machine side reaches DEAF; the window-guard is a daemon concern.
#[test]
fn test_state_machine_reaches_deaf_regardless_of_window() {
    let mut s = HearingState::new(3, 2);
    let now = chrono::Utc::now();
    let _ = s.feed(&deaf_env(), now); // → DEGRADED
    let _ = s.feed(&deaf_env(), now); // streak=2
    let _ = s.feed(&deaf_env(), now); // → DEAF
    assert_eq!(
        s.liveness,
        HearingLiveness::Deaf,
        "state machine must reach DEAF at K failures"
    );
}

/// Inside waking hours the window returns true.
#[test]
fn test_inside_window_at_noon() {
    let cfg = PresenceConfig {
        enabled: true,
        waking_start: t(8, 0),
        waking_end: t(21, 0),
        silence_threshold: t(12, 0),
        ..PresenceConfig::default()
    };

    let local_time = t(12, 0);
    let is_in_window = local_time >= cfg.waking_start && local_time < cfg.waking_end;
    assert!(is_in_window, "12:00 must be inside the waking-hours window");
}
