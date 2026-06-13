//! `wintermute-presence` — privacy-first presence heartbeat daemon.
//!
//! Knows she's okay without watching her.
//!
//! # Privacy invariant
//!
//! This crate never stores, logs, or forwards transcript text. It extracts
//! only the **byte count** of a `wm.stt.final` transcript and publishes that
//! count as `transcript_len`.

pub mod config;
pub mod daemon;
pub mod events;
pub mod hearing;
pub mod silence;
pub mod state;
pub mod status;
