//! Hearing AC4: per-window `hearing_confirmed_in_window` persists across a
//! daemon restart (write-then-reload test, mirroring state persistence).

use chrono::NaiveDate;
use wintermute_presence::hearing::HearingLiveness;
use wintermute_presence::state::{DailyState, StateStore};

#[tokio::test]
async fn test_hearing_confirmed_persists_across_restart() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state.json");
    let store = StateStore::new(path.clone());

    let today = chrono::Local::now().date_naive();

    // Write a state with hearing_confirmed_in_window = true.
    let written = DailyState {
        date: today,
        daily_count: 0,
        last_interaction_ts: None,
        silence_emitted_for_window: false,
        hearing_confirmed_in_window: true,
        hearing_liveness: HearingLiveness::Hearing,
        hearing_last_ok_ts: Some(chrono::Utc::now()),
    };
    store.save(&written).await?;

    // Reload (simulates daemon restart).
    let loaded = store.load().await?;
    assert_eq!(loaded.date, today);
    assert!(
        loaded.hearing_confirmed_in_window,
        "hearing_confirmed_in_window must survive a daemon restart"
    );
    assert_eq!(
        loaded.hearing_liveness,
        HearingLiveness::Hearing,
        "hearing_liveness must survive a daemon restart"
    );
    assert!(
        loaded.hearing_last_ok_ts.is_some(),
        "hearing_last_ok_ts must survive a daemon restart"
    );
    Ok(())
}

#[tokio::test]
async fn test_hearing_liveness_deaf_persists() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state.json");
    let store = StateStore::new(path.clone());

    let today = chrono::Local::now().date_naive();

    let written = DailyState {
        date: today,
        daily_count: 0,
        last_interaction_ts: None,
        silence_emitted_for_window: false,
        hearing_confirmed_in_window: false,
        hearing_liveness: HearingLiveness::Deaf,
        hearing_last_ok_ts: None,
    };
    store.save(&written).await?;

    let loaded = store.load().await?;
    assert_eq!(
        loaded.hearing_liveness,
        HearingLiveness::Deaf,
        "DEAF liveness must be preserved across restart"
    );
    Ok(())
}

#[test]
fn test_fresh_state_defaults_hearing() {
    // A fresh state should default to hearing_confirmed_in_window=false
    // and hearing_liveness=Hearing (optimistic).
    let today = NaiveDate::from_ymd_opt(2026, 6, 18).unwrap_or_default();
    let state = DailyState::fresh(today);
    assert!(!state.hearing_confirmed_in_window);
    assert_eq!(state.hearing_liveness, HearingLiveness::Hearing);
    assert!(state.hearing_last_ok_ts.is_none());
}
