//! AC8 (MUST): `wm-presence status` prints today's count and last-interaction timestamp.

use std::path::PathBuf;
use wintermute_presence::state::{DailyState, StateStore};
use wintermute_presence::status::format_status;

#[tokio::test]
async fn test_status_output() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state.json");
    let store = StateStore::new(path);

    let ts = chrono::DateTime::parse_from_rfc3339("2026-05-30T14:23:00Z")
        .map(|dt| dt.with_timezone(&chrono::Utc))?;

    let state = DailyState {
        date: chrono::NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid"),
        daily_count: 7,
        last_interaction_ts: Some(ts),
        silence_emitted_for_window: false,
    };
    store.save(&state).await?;

    let loaded = store.load().await?;
    let output = format_status(&loaded);

    assert!(
        output.contains("7"),
        "status must contain today's count (7); got: {output}"
    );
    assert!(
        output.contains("2026-05-30") || output.contains("14:23"),
        "status must contain last-interaction timestamp; got: {output}"
    );
    Ok(())
}
