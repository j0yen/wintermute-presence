//! AC2 (MUST): A `wm.stt.final { transcript: "hello there" }` produces
//! `wm.presence.summon { transcript_len: 11 }`.
//! The daemon must not emit any substring of the transcript text.

use wintermute_presence::daemon::MockBusHandle;
use wintermute_presence::events::{TOPIC_PRESENCE_SUMMON, TOPIC_STT_FINAL};

const TRANSCRIPT: &str = "hello there";

#[tokio::test]
async fn test_stt_final_transcript_len_not_text() {
    let handle = MockBusHandle::new();

    handle
        .inject_event(
            TOPIC_STT_FINAL,
            serde_json::json!({ "transcript": TRANSCRIPT }),
        )
        .await;

    handle.run_once_enabled().await;

    let published = handle.drain_published().await;

    // Find the summon event.
    let summon = published
        .iter()
        .find(|(topic, _)| topic == TOPIC_PRESENCE_SUMMON)
        .unwrap_or_else(|| panic!("no wm.presence.summon published; got: {published:?}"));

    let payload = &summon.1;

    // transcript_len must equal TRANSCRIPT.len().
    let len = payload
        .get("transcript_len")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| panic!("transcript_len missing in {payload}"));
    assert_eq!(
        len,
        TRANSCRIPT.len() as u64,
        "transcript_len must equal byte count of transcript"
    );

    // The transcript text must not appear anywhere in the serialized payload.
    let payload_str = serde_json::to_string(payload).unwrap_or_default();
    assert!(
        !payload_str.contains(TRANSCRIPT),
        "transcript text leaked into payload: {payload_str}"
    );

    // Check the full egress: no published message may contain the transcript.
    for (topic, data) in &published {
        let data_str = serde_json::to_string(data).unwrap_or_default();
        assert!(
            !data_str.contains(TRANSCRIPT),
            "transcript text found in topic={topic} payload={data_str}"
        );
    }
}
