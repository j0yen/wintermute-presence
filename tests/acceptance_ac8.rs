//! AC8 (MUST): `wm-presence status` prints today's count and last-interaction timestamp.

use wintermute_presence::state::{DailyState, StateStore};
use wintermute_presence::status::format_status;

#[tokio::test]
async fn test_status_output() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state.json");
    let store = StateStore::new(path);

    let today = chrono::Local::now().date_naive();
    let ts = chrono::Utc::now();

    let state = DailyState {
        date: today,
        daily_count: 7,
        last_interaction_ts: Some(ts),
        silence_emitted_for_window: false,
        hearing_confirmed_in_window: false,
        ..DailyState::fresh(today)
    };
    store.save(&state).await?;

    let loaded = store.load().await?;
    let output = format_status(&loaded);

    assert!(
        output.contains("7"),
        "status must contain today's count (7); got: {output}"
    );
    assert!(
        output.contains("interactions=7"),
        "status must contain interactions count; got: {output}"
    );
    assert!(
        output.contains("hearing_liveness="),
        "status must include hearing_liveness; got: {output}"
    );
    assert!(
        output.contains("hearing_last_ok="),
        "status must include hearing_last_ok; got: {output}"
    );
    Ok(())
}
