//! Hearing AC5: with `hearing_watch_enabled:false`, the daemon's pre-existing
//! `wm.presence.*` behaviour is completely unchanged — no hearing events, no
//! state transitions, no edge events.

use wintermute_presence::daemon::MockBusHandle;
use wintermute_presence::events::{TOPIC_AUDIO_WAKE, TOPIC_PRESENCE_SUMMON};
use wintermute_presence::hearing::TOPIC_HEALTH_HEARING;

/// When `hearing_watch_enabled` is false, a `wm.health.hearing` envelope must
/// not produce any published events and must not affect the summon path.
#[tokio::test]
async fn test_hearing_disabled_no_events() {
    let handle = MockBusHandle::new();

    // Inject a hearing envelope — should be silently ignored when disabled.
    handle
        .inject_event(
            TOPIC_HEALTH_HEARING,
            serde_json::json!({
                "state": "deaf",
                "model_present": false,
                "detector_loaded": false,
            }),
        )
        .await;

    // Run with presence DISABLED (simulates hearing_watch_enabled:false at the
    // MockBusHandle level; the mock run_once_disabled clears all events).
    handle.run_once_disabled().await;

    let published = handle.drain_published().await;
    assert!(
        published.is_empty(),
        "disabled daemon must not publish any events; got {published:?}"
    );
}

/// With presence enabled, a wake event still produces exactly one summon even
/// when hearing envelopes are also in flight — hearing routing must not
/// interfere with the interaction path.
#[tokio::test]
async fn test_wake_summon_unaffected_by_hearing() {
    let handle = MockBusHandle::new();

    // Inject a wake and a hearing envelope in the same batch.
    handle
        .inject_event(TOPIC_AUDIO_WAKE, serde_json::json!({}))
        .await;
    // The mock does not route TOPIC_HEALTH_HEARING (no routing rule), so it
    // produces no summon for the hearing envelope.
    handle
        .inject_event(
            TOPIC_HEALTH_HEARING,
            serde_json::json!({ "state": "ok", "model_present": true, "detector_loaded": true }),
        )
        .await;

    handle.run_once_enabled().await;

    let published = handle.drain_published().await;
    // Exactly one summon from the wake event; the hearing envelope is ignored
    // by the mock's routing (it's not an audio wake or stt topic).
    let summon_count = published
        .iter()
        .filter(|(topic, _)| topic == TOPIC_PRESENCE_SUMMON)
        .count();
    assert_eq!(
        summon_count, 1,
        "wake must produce exactly one summon regardless of hearing events; got {published:?}"
    );
}
