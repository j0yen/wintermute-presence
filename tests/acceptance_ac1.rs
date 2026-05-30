//! AC1 (MUST): With presence enabled, a published `wm.audio.wake` produces
//! exactly one `wm.presence.summon` on the bus.
//!
//! Integration test using the mock bus infrastructure from tests/mocks/ac1.rs.

use wintermute_presence::daemon::MockBusHandle;
use wintermute_presence::events::{TOPIC_AUDIO_WAKE, TOPIC_PRESENCE_SUMMON};

#[tokio::test]
async fn test_wake_produces_summon() {
    let handle = MockBusHandle::new();

    // Inject a wm.audio.wake event with presence enabled.
    handle.inject_event(TOPIC_AUDIO_WAKE, serde_json::json!({})).await;

    // Run one iteration of the daemon event loop.
    handle.run_once_enabled().await;

    // Exactly one wm.presence.summon must have been published.
    let published = handle.drain_published().await;
    let summons: Vec<_> = published
        .iter()
        .filter(|(topic, _)| topic == TOPIC_PRESENCE_SUMMON)
        .collect();
    assert_eq!(
        summons.len(),
        1,
        "expected exactly 1 wm.presence.summon, got {}: {published:?}",
        summons.len()
    );
}
