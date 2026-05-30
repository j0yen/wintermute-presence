//! Topic constants and payload types for the presence bus protocol.
//!
//! Privacy invariant: no payload type in this module contains a `transcript`
//! field or any other text from a user utterance.

use serde::{Deserialize, Serialize};

/// Topic emitted by the audio wake-word detector.
pub const TOPIC_AUDIO_WAKE: &str = "wm.audio.wake";

/// Topic emitted by the speech-to-text engine (carries transcript text).
pub const TOPIC_STT_FINAL: &str = "wm.stt.final";

/// Topic emitted by this daemon: one per interaction.
pub const TOPIC_PRESENCE_SUMMON: &str = "wm.presence.summon";

/// Topic emitted by this daemon: one per silence window (debounced).
pub const TOPIC_PRESENCE_SILENCE: &str = "wm.presence.silence";

/// Prefix for all topics this daemon emits (used for self-filter).
pub const TOPIC_PRESENCE_PREFIX: &str = "wm.presence.";

/// Payload for [`TOPIC_PRESENCE_SUMMON`].
///
/// Privacy: `transcript_len` is a byte count only — never the text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummonPayload {
    /// ISO-8601 UTC timestamp of the interaction.
    pub ts: chrono::DateTime<chrono::Utc>,
    /// Byte count of the transcript, or 0 for a bare wake event.
    pub transcript_len: usize,
}

/// Payload for [`TOPIC_PRESENCE_SILENCE`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilencePayload {
    /// When the window started (ISO-8601 UTC).
    pub since_ts: chrono::DateTime<chrono::Utc>,
    /// Human-readable window description (e.g. "08:00–21:00 local").
    pub window: String,
}

/// Fields we need from an incoming `wm.stt.final` payload.
///
/// We intentionally do NOT derive `Debug` here to prevent accidental
/// transcript text leaking into log output.
#[derive(Deserialize)]
pub struct SttFinalPayload {
    /// The transcript text — extracted for length only; never forwarded.
    #[serde(default)]
    pub transcript: String,
}

impl SttFinalPayload {
    /// Return the transcript's byte count, consuming `self` so the text cannot
    /// be forwarded.
    #[must_use]
    pub fn transcript_len(self) -> usize {
        self.transcript.len()
    }
}
