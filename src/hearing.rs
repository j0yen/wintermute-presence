//! Hearing-liveness state machine for wintermute-presence.
//!
//! Subscribes to `wm.health.hearing` envelopes, maintains a
//! `HEARING / DEGRADED / DEAF` state machine, and emits edge events
//! (`wm.health.hearing.fail` / `wm.health.hearing.ok`) on state transitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public topic constants
// ---------------------------------------------------------------------------

/// Incoming topic from pulse-hearing-probe (or any selftest emitter).
pub const TOPIC_HEALTH_HEARING: &str = "wm.health.hearing";

/// Emitted once on the `→ Deaf` edge transition.
pub const TOPIC_HEALTH_HEARING_FAIL: &str = "wm.health.hearing.fail";

/// Emitted once on the `Deaf → Hearing` recovery edge.
pub const TOPIC_HEALTH_HEARING_OK: &str = "wm.health.hearing.ok";

// ---------------------------------------------------------------------------
// Envelope from the bus
// ---------------------------------------------------------------------------

/// Payload shape of an incoming `wm.health.hearing` message.
///
/// Only the fields we need are declared; unknown fields are ignored.
#[derive(Debug, Deserialize, Default)]
pub struct HearingEnvelope {
    /// Overall verdict: `"ok"`, `"degraded"`, or `"deaf"`.
    #[serde(default)]
    pub state: String,
    /// If `false`, forces immediate `Deaf` (structural fault).
    #[serde(default = "default_true")]
    pub model_present: bool,
    /// If `false`, forces immediate `Deaf` (structural fault).
    #[serde(default = "default_true")]
    pub detector_loaded: bool,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

/// Liveness state of the hearing system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HearingLiveness {
    /// System is confirmed hearing (K+ ok probes without a failure run).
    Hearing,
    /// One or more consecutive failures, but below the deaf threshold.
    Degraded,
    /// K consecutive failures — emit `wm.health.hearing.fail`.
    Deaf,
    /// In recovery from Deaf: has seen consecutive ok probes (internal).
    /// Serialized so it persists across restarts.
    Healing,
}

impl Default for HearingLiveness {
    fn default() -> Self {
        Self::Hearing
    }
}

impl HearingLiveness {
    /// Human-readable label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hearing => "HEARING",
            Self::Degraded => "DEGRADED",
            Self::Deaf => "DEAF",
            Self::Healing => "HEALING",
        }
    }
}

// ---------------------------------------------------------------------------
// Transition output
// ---------------------------------------------------------------------------

/// What the caller should publish (if anything) after a state transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HearingEdge {
    /// No edge event needed.
    None,
    /// Crossed into `Deaf` — emit `wm.health.hearing.fail`.
    BecameDeaf,
    /// Recovered out of `Deaf` — emit `wm.health.hearing.ok`.
    Recovered,
}

// ---------------------------------------------------------------------------
// State machine logic
// ---------------------------------------------------------------------------

/// In-memory state of the hearing watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HearingState {
    /// Current liveness verdict.
    pub liveness: HearingLiveness,
    /// Number of consecutive non-ok probes (resets to 0 on ok).
    pub failure_streak: u32,
    /// Number of consecutive ok probes while HEALING (resets on non-ok).
    pub healing_streak: u32,
    /// UTC timestamp of the last `ok` probe received.
    pub last_ok_ts: Option<DateTime<Utc>>,
    /// K consecutive failures before → DEAF.
    pub deaf_threshold_k: u32,
    /// K consecutive ok probes while HEALING before → HEARING.
    pub healing_threshold_k: u32,
}

impl HearingState {
    /// Create initial state with the given thresholds.
    #[must_use]
    pub fn new(deaf_threshold_k: u32, healing_threshold_k: u32) -> Self {
        Self {
            liveness: HearingLiveness::Hearing,
            failure_streak: 0,
            healing_streak: 0,
            last_ok_ts: None,
            deaf_threshold_k,
            healing_threshold_k,
        }
    }

    /// Feed one probe result into the state machine.
    ///
    /// Returns the edge event (if any) that must be published.
    ///
    /// # Parameters
    ///
    /// * `envelope` — deserialized envelope from `wm.health.hearing`.
    /// * `now`      — current UTC time (for `last_ok_ts`).
    #[must_use]
    pub fn feed(&mut self, envelope: &HearingEnvelope, now: DateTime<Utc>) -> HearingEdge {
        // Structural fault → immediate Deaf.
        if !envelope.model_present || !envelope.detector_loaded {
            return self.force_deaf();
        }

        let is_ok = envelope.state == "ok";

        match self.liveness {
            HearingLiveness::Hearing => {
                if is_ok {
                    self.failure_streak = 0;
                    self.last_ok_ts = Some(now);
                    HearingEdge::None
                } else {
                    self.failure_streak = 1;
                    self.liveness = HearingLiveness::Degraded;
                    HearingEdge::None
                }
            }
            HearingLiveness::Degraded => {
                if is_ok {
                    self.failure_streak = 0;
                    self.liveness = HearingLiveness::Hearing;
                    self.last_ok_ts = Some(now);
                    HearingEdge::None
                } else {
                    self.failure_streak = self.failure_streak.saturating_add(1);
                    if self.failure_streak >= self.deaf_threshold_k {
                        self.liveness = HearingLiveness::Deaf;
                        HearingEdge::BecameDeaf
                    } else {
                        HearingEdge::None
                    }
                }
            }
            HearingLiveness::Deaf => {
                if is_ok {
                    self.failure_streak = 0;
                    self.last_ok_ts = Some(now);
                    self.healing_streak = self.healing_streak.saturating_add(1);
                    if self.healing_streak >= self.healing_threshold_k {
                        // Immediate recovery without visiting Healing state visibly.
                        self.liveness = HearingLiveness::Hearing;
                        self.healing_streak = 0;
                        HearingEdge::Recovered
                    } else {
                        self.liveness = HearingLiveness::Healing;
                        HearingEdge::None
                    }
                } else {
                    // Stay DEAF — no additional edge event.
                    HearingEdge::None
                }
            }
            HearingLiveness::Healing => {
                if is_ok {
                    self.healing_streak = self.healing_streak.saturating_add(1);
                    self.last_ok_ts = Some(now);
                    if self.healing_streak >= self.healing_threshold_k {
                        self.liveness = HearingLiveness::Hearing;
                        self.healing_streak = 0;
                        self.failure_streak = 0;
                        HearingEdge::Recovered
                    } else {
                        HearingEdge::None
                    }
                } else {
                    // Healing reset — back to DEAF.
                    self.healing_streak = 0;
                    self.failure_streak = self.failure_streak.saturating_add(1);
                    self.liveness = HearingLiveness::Deaf;
                    // Already emitted fail when entering Deaf; don't re-emit.
                    HearingEdge::None
                }
            }
        }
    }

    /// Force immediate `Deaf` regardless of current state.
    fn force_deaf(&mut self) -> HearingEdge {
        if self.liveness == HearingLiveness::Deaf {
            return HearingEdge::None;
        }
        self.liveness = HearingLiveness::Deaf;
        self.failure_streak = self.deaf_threshold_k;
        self.healing_streak = 0;
        HearingEdge::BecameDeaf
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    fn ok_env() -> HearingEnvelope {
        HearingEnvelope {
            state: "ok".to_string(),
            model_present: true,
            detector_loaded: true,
        }
    }

    fn deaf_env() -> HearingEnvelope {
        HearingEnvelope {
            state: "deaf".to_string(),
            model_present: true,
            detector_loaded: true,
        }
    }

    fn degraded_env() -> HearingEnvelope {
        HearingEnvelope {
            state: "degraded".to_string(),
            model_present: true,
            detector_loaded: true,
        }
    }

    // AC1: K consecutive non-ok drives to DEAF exactly at Kth failure.
    #[test]
    fn ac1_k_consecutive_failures_trigger_deaf() {
        let k = 3;
        let mut s = HearingState::new(k, 2);
        // first failure: HEARING → DEGRADED
        let e = s.feed(&deaf_env(), now());
        assert_eq!(s.liveness, HearingLiveness::Degraded);
        assert_eq!(e, HearingEdge::None);
        // second failure: DEGRADED, streak=2, below K
        let e = s.feed(&deaf_env(), now());
        assert_eq!(s.liveness, HearingLiveness::Degraded);
        assert_eq!(e, HearingEdge::None);
        // third failure: DEGRADED → DEAF (streak == K)
        let e = s.feed(&deaf_env(), now());
        assert_eq!(s.liveness, HearingLiveness::Deaf);
        assert_eq!(e, HearingEdge::BecameDeaf);
    }

    // AC1: K-1 failures does NOT trigger DEAF.
    #[test]
    fn ac1_k_minus_one_failures_no_deaf() {
        let k = 3;
        let mut s = HearingState::new(k, 2);
        let _ = s.feed(&deaf_env(), now()); // → DEGRADED, streak=1
        let e = s.feed(&deaf_env(), now()); // streak=2 (K-1)
        assert_eq!(s.liveness, HearingLiveness::Degraded);
        assert_eq!(e, HearingEdge::None);
    }

    // AC1: single ok returns from DEAF back to HEARING (via healing).
    #[test]
    fn ac1_ok_from_deaf_recovers() {
        let mut s = HearingState::new(2, 1);
        let _ = s.feed(&deaf_env(), now()); // → DEGRADED
        let _ = s.feed(&deaf_env(), now()); // → DEAF
        // Single ok should → HEALING; with healing_k=1 → HEARING + Recovered
        let e = s.feed(&ok_env(), now());
        assert_eq!(s.liveness, HearingLiveness::Hearing);
        assert_eq!(e, HearingEdge::Recovered);
    }

    // AC2: model_present:false forces immediate DEAF.
    #[test]
    fn ac2_model_absent_forces_deaf() {
        let mut s = HearingState::new(3, 2);
        let env = HearingEnvelope {
            state: "ok".to_string(),
            model_present: false,
            detector_loaded: true,
        };
        let e = s.feed(&env, now());
        assert_eq!(s.liveness, HearingLiveness::Deaf);
        assert_eq!(e, HearingEdge::BecameDeaf);
    }

    // AC2: detector_loaded:false forces immediate DEAF.
    #[test]
    fn ac2_detector_unloaded_forces_deaf() {
        let mut s = HearingState::new(3, 2);
        let env = HearingEnvelope {
            state: "ok".to_string(),
            model_present: true,
            detector_loaded: false,
        };
        let e = s.feed(&env, now());
        assert_eq!(s.liveness, HearingLiveness::Deaf);
        assert_eq!(e, HearingEdge::BecameDeaf);
    }

    // AC3: fail emitted exactly once on → DEAF edge; re-entering stays DEAF, no event.
    #[test]
    fn ac3_fail_emitted_exactly_once_on_deaf_edge() {
        let mut s = HearingState::new(2, 2);
        let _ = s.feed(&deaf_env(), now()); // DEGRADED
        let e1 = s.feed(&deaf_env(), now()); // → DEAF
        assert_eq!(e1, HearingEdge::BecameDeaf);
        // Additional deaf probes while already DEAF.
        let e2 = s.feed(&deaf_env(), now());
        let e3 = s.feed(&deaf_env(), now());
        assert_eq!(e2, HearingEdge::None);
        assert_eq!(e3, HearingEdge::None);
    }

    // AC3: ok emitted exactly once on recovery.
    #[test]
    fn ac3_ok_emitted_exactly_once_on_recovery() {
        let mut s = HearingState::new(2, 2);
        let _ = s.feed(&deaf_env(), now());
        let _ = s.feed(&deaf_env(), now()); // → DEAF
        let _ = s.feed(&ok_env(), now()); // → HEALING streak=1
        let e = s.feed(&ok_env(), now()); // → HEARING + Recovered
        assert_eq!(e, HearingEdge::Recovered);
        // Extra ok while already HEARING → None.
        let e2 = s.feed(&ok_env(), now());
        assert_eq!(e2, HearingEdge::None);
    }

    // AC3: re-entering HEARING from HEARING on ok is not a recovery event.
    #[test]
    fn ac3_no_spurious_recovery_in_hearing() {
        let mut s = HearingState::new(3, 2);
        let e = s.feed(&ok_env(), now());
        assert_eq!(e, HearingEdge::None);
    }

    // Degraded ok clears back to HEARING without spurious events.
    #[test]
    fn degraded_ok_clears_to_hearing() {
        let mut s = HearingState::new(3, 2);
        let _ = s.feed(&degraded_env(), now()); // → DEGRADED
        let e = s.feed(&ok_env(), now()); // → HEARING
        assert_eq!(s.liveness, HearingLiveness::Hearing);
        assert_eq!(e, HearingEdge::None);
    }
}
