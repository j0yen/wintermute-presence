//! AC3 (MUST): The daily counter survives a daemon restart.
//! State file written before exit is read correctly on next start.

use std::path::PathBuf;
use wintermute_presence::state::{DailyState, StateStore};

#[tokio::test]
async fn test_state_file_round_trip() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state.json");
    let store = StateStore::new(path.clone());

    // Write a state.
    let written = DailyState {
        date: chrono::Local::now().date_naive(),
        daily_count: 42,
        last_interaction_ts: Some(chrono::Utc::now()),
        silence_emitted_for_window: false,
    };
    store.save(&written).await?;

    // Read it back — simulating a daemon restart.
    let read = store.load().await?;
    assert_eq!(
        read.daily_count, 42,
        "daily_count not preserved across restart"
    );
    assert!(!read.silence_emitted_for_window);
    assert!(read.last_interaction_ts.is_some());
    Ok(())
}
