//! AC9 (MUST): The daemon applies the self-emitted-topic filter and does not
//! consume its own `wm.presence.*` events.

use wintermute_presence::daemon::MockBusHandle;
use wintermute_presence::events::TOPIC_PRESENCE_SUMMON;

#[tokio::test]
async fn test_self_topic_filter() {
    let handle = MockBusHandle::new();

    // Inject a wm.presence.summon event as if it came from ourselves.
    // The daemon must ignore this entirely (no double-emit).
    handle
        .inject_self_event(TOPIC_PRESENCE_SUMMON, serde_json::json!({ "transcript_len": 0 }))
        .await;

    handle.run_once_enabled().await;

    // No additional wm.presence.* events should have been published.
    let published = handle.drain_published().await;
    assert!(
        published.is_empty(),
        "daemon must not re-publish its own presence events; got: {published:?}"
    );
}
