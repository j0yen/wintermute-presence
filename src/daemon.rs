//! Daemon: subscribe to wm.audio and wm.stt, emit wm.presence.*.
//!
//! The real daemon connects to the agorabus socket. Tests use
//! [`MockBusHandle`] which exercises the same logic paths with an
//! in-memory channel.

#![allow(clippy::future_not_send)]

use anyhow::{Context as _, Result};
use chrono::{Local, Utc};
use serde_json::Value;
use std::path::Path;
use tracing::{error, info, warn};

use crate::config::PresenceConfig;
use crate::events::{
    SttFinalPayload, SummonPayload, TOPIC_AUDIO_WAKE, TOPIC_PRESENCE_PREFIX,
    TOPIC_PRESENCE_SUMMON, TOPIC_STT_FINAL,
};
use crate::hearing::{
    HearingEdge, HearingEnvelope, HearingState, TOPIC_HEALTH_HEARING, TOPIC_HEALTH_HEARING_FAIL,
    TOPIC_HEALTH_HEARING_OK,
};
use crate::state::{DailyState, StateStore};

/// Run the presence daemon against a real agorabus socket.
///
/// Blocks until the bus disconnects or an unrecoverable error occurs.
///
/// # Errors
///
/// Returns `Err` on bus connect failure or I/O error.
pub async fn run(sock: &Path, cfg: &PresenceConfig, store: &StateStore) -> Result<()> {
    if !cfg.enabled {
        info!("presence disabled in config — daemon is a no-op");
        return Ok(());
    }

    let pid = std::process::id();
    let session_id = format!("wm-presence-daemon-{pid}");

    let mut client = connect_and_subscribe(sock, &session_id, pid).await?;
    let mut state = store.load().await.context("loading state")?;

    // Hearing watcher state — only allocated when enabled.
    let mut hearing = if cfg.hearing_watch_enabled {
        Some(HearingState::new(
            cfg.hearing_deaf_threshold_k,
            cfg.hearing_healing_threshold_k,
        ))
    } else {
        None
    };

    loop {
        let event = match client.next_event().await {
            Ok(Some(e)) => e,
            Ok(None) => {
                info!("agorabus connection closed — daemon exiting");
                break;
            }
            Err(e) => {
                error!(?e, "error reading from agorabus");
                break;
            }
        };

        // Self-filter: skip events from ourselves or our own topics.
        if event.from == session_id || event.topic.starts_with(TOPIC_PRESENCE_PREFIX) {
            continue;
        }

        // Route hearing envelopes first (before the interaction router).
        if event.topic == TOPIC_HEALTH_HEARING {
            if let Some(ref mut hs) = hearing {
                let now = Utc::now();
                let in_window = is_in_waking_window(cfg);
                let envelope: HearingEnvelope =
                    serde_json::from_value(event.data.clone()).unwrap_or_default();
                let edge = hs.feed(&envelope, now);

                // Update per-window hearing confirmation and persist liveness.
                if envelope.state == "ok" && in_window {
                    state = state.with_hearing_confirmed();
                }
                // Always persist liveness after each envelope so status + restarts see it.
                state = state.with_hearing_liveness(hs.liveness, hs.last_ok_ts);
                if let Err(e) = store.save(&state).await {
                    error!(?e, "failed to save state after hearing envelope");
                }

                match edge {
                    HearingEdge::BecameDeaf if in_window => {
                        if let Err(e) =
                            publish_signal(sock, TOPIC_HEALTH_HEARING_FAIL, pid).await
                        {
                            error!(?e, "failed to publish wm.health.hearing.fail");
                        }
                    }
                    HearingEdge::Recovered => {
                        if let Err(e) =
                            publish_signal(sock, TOPIC_HEALTH_HEARING_OK, pid).await
                        {
                            error!(?e, "failed to publish wm.health.hearing.ok");
                        }
                    }
                    _ => {}
                }
            }
            continue;
        }

        // Route the event to interaction processing.
        let Some(transcript_len) = route_topic(&event.topic, &event.data) else {
            continue; // Unrecognized topic — ignore.
        };

        let (new_state, summon) = process_interaction(state, transcript_len);
        state = new_state;

        if let Err(e) = publish_summon(sock, &summon).await {
            error!(?e, "failed to publish wm.presence.summon");
        }
        if let Err(e) = store.save(&state).await {
            error!(?e, "failed to save state");
        }
    }

    Ok(())
}

/// Returns `true` if the current local time is within the configured waking-hours window.
#[must_use]
fn is_in_waking_window(cfg: &PresenceConfig) -> bool {
    let local_now = Local::now().time();
    local_now >= cfg.waking_start && local_now < cfg.waking_end
}

/// Publish a bare signal (empty JSON object) to `topic` on the bus.
async fn publish_signal(sock: &Path, topic: &str, pid: u32) -> Result<()> {
    let pub_session = format!("wm-presence-pub-{pid}");
    let mut client = agorabus::Client::connect(sock)
        .await
        .context("connecting publish client")?;
    client
        .announce(&pub_session, pid, "/", "wm-presence publish")
        .await
        .context("announcing publish client")?;
    client
        .publish(topic, Value::Object(serde_json::Map::new()))
        .await
        .with_context(|| format!("publishing {topic}"))?;
    Ok(())
}

/// Trigger a hearing probe via `wm-audio selftest --emit` (fire-and-forget).
///
/// The response arrives asynchronously as a `wm.health.hearing` envelope on
/// the bus, which the main event loop processes.
///
/// # Errors
///
/// Logs a warning but does not propagate errors — a failed probe counts as a
/// missing response, which the timeout path handles.
pub fn spawn_hearing_probe() {
    match std::process::Command::new("wm-audio")
        .args(["selftest", "--emit"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_child) => {} // child runs independently; we don't wait.
        Err(e) => {
            warn!(?e, "failed to spawn wm-audio selftest --emit");
        }
    }
}

/// Connect to the bus, announce, and subscribe to `wm.`.
async fn connect_and_subscribe(
    sock: &Path,
    session_id: &str,
    pid: u32,
) -> Result<agorabus::Client> {
    let mut client = agorabus::Client::connect(sock)
        .await
        .context("connecting to agorabus")?;
    client
        .announce(session_id, pid, "/", "wm-presence daemon")
        .await
        .context("announcing to agorabus")?;
    // Subscribe to wm. prefix — catches wm.audio.wake and wm.stt.final.
    client
        .subscribe("wm.")
        .await
        .context("subscribing to wm.")?;
    Ok(client)
}

/// Route an event topic to a `transcript_len`. Returns `None` for unrecognized topics.
#[must_use]
fn route_topic(topic: &str, data: &Value) -> Option<usize> {
    if topic == TOPIC_STT_FINAL {
        Some(extract_transcript_len(data))
    } else if topic == TOPIC_AUDIO_WAKE {
        Some(0)
    } else {
        None
    }
}

/// Open a short-lived publish connection to emit `wm.presence.summon`.
async fn publish_summon(sock: &Path, summon: &SummonPayload) -> Result<()> {
    let pid = std::process::id();
    let pub_session = format!("wm-presence-pub-{pid}");
    let mut client = agorabus::Client::connect(sock)
        .await
        .context("connecting publish client")?;
    client
        .announce(&pub_session, pid, "/", "wm-presence publish")
        .await
        .context("announcing publish client")?;
    let payload = serde_json::to_value(summon).unwrap_or(Value::Null);
    client
        .publish(TOPIC_PRESENCE_SUMMON, payload)
        .await
        .context("publishing wm.presence.summon")?;
    Ok(())
}

/// Extract `transcript_len` from a `wm.stt.final` payload.
///
/// Privacy: only the byte count is returned — the text is consumed and dropped.
///
/// Returns 0 if the payload cannot be parsed.
#[must_use]
pub fn extract_transcript_len(data: &Value) -> usize {
    serde_json::from_value::<SttFinalPayload>(data.clone())
        .map(SttFinalPayload::transcript_len)
        .unwrap_or(0)
}

/// Process one interaction event, returning the updated state and the summon
/// payload to publish.
///
/// `transcript_len` is 0 for a bare wake event.
#[must_use]
pub fn process_interaction(state: DailyState, transcript_len: usize) -> (DailyState, SummonPayload) {
    let ts = Utc::now();
    let summon = SummonPayload { ts, transcript_len };
    let new_state = state.with_interaction(ts);
    (new_state, summon)
}

// ---------------------------------------------------------------------------
// Mock infrastructure for integration tests
// ---------------------------------------------------------------------------

/// In-memory bus handle for tests.
///
/// Allows injecting events and inspecting publications without a real socket.
pub struct MockBusHandle {
    /// Events injected by the test.
    inbound: tokio::sync::Mutex<Vec<(String, String, Value)>>,
    /// Events published by the daemon under test.
    published: tokio::sync::Mutex<Vec<(String, Value)>>,
    /// Session id of the daemon (for self-filter).
    pub session_id: String,
}

impl MockBusHandle {
    /// Create a new handle with a fixed session id.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inbound: tokio::sync::Mutex::new(Vec::new()),
            published: tokio::sync::Mutex::new(Vec::new()),
            session_id: "wm-presence-daemon-test".to_string(),
        }
    }

    /// Inject an event from an external source (not ourselves).
    pub async fn inject_event(&self, topic: &str, data: Value) {
        self.inbound
            .lock()
            .await
            .push((topic.to_string(), "external-peer".to_string(), data));
    }

    /// Inject an event from ourselves (self-filter test).
    pub async fn inject_self_event(&self, topic: &str, data: Value) {
        self.inbound
            .lock()
            .await
            .push((topic.to_string(), self.session_id.clone(), data));
    }

    /// Drain all published events and return them.
    pub async fn drain_published(&self) -> Vec<(String, Value)> {
        let mut guard = self.published.lock().await;
        std::mem::take(&mut *guard)
    }

    /// Run one iteration of the event loop with presence ENABLED.
    pub async fn run_once_enabled(&self) {
        self.run_once(true).await;
    }

    /// Run one iteration of the event loop with presence DISABLED.
    pub async fn run_once_disabled(&self) {
        self.run_once(false).await;
    }

    async fn run_once(&self, enabled: bool) {
        if !enabled {
            // Consume all inbound events without publishing.
            self.inbound.lock().await.clear();
            return;
        }

        let events: Vec<(String, String, Value)> = {
            let mut guard = self.inbound.lock().await;
            std::mem::take(&mut *guard)
        };

        for (topic, from, data) in events {
            // Self-filter: skip our own session_id and our own topics.
            if from == self.session_id || topic.starts_with(TOPIC_PRESENCE_PREFIX) {
                continue;
            }

            let Some(transcript_len) = route_topic(&topic, &data) else {
                continue;
            };

            let summon = SummonPayload {
                ts: Utc::now(),
                transcript_len,
            };
            let payload = serde_json::to_value(&summon).unwrap_or(Value::Null);
            self.published
                .lock()
                .await
                .push((TOPIC_PRESENCE_SUMMON.to_string(), payload));
        }
    }
}

impl Default for MockBusHandle {
    fn default() -> Self {
        Self::new()
    }
}
