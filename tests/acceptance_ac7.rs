//! AC7 (MUST): With presence disabled in config, the daemon subscribes nothing
//! and emits nothing — verified by an empty bus after wake events.

use wintermute_presence::daemon::MockBusHandle;
use wintermute_presence::events::TOPIC_AUDIO_WAKE;

#[tokio::test]
async fn test_opt_in_gate_disabled() {
    let handle = MockBusHandle::new();

    // Inject a wm.audio.wake event.
    handle.inject_event(TOPIC_AUDIO_WAKE, serde_json::json!({})).await;

    // Run one iteration with presence DISABLED.
    handle.run_once_disabled().await;

    // No events should have been published.
    let published = handle.drain_published().await;
    assert!(
        published.is_empty(),
        "disabled daemon must not emit anything; got: {published:?}"
    );
}
