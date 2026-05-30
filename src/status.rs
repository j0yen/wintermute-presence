//! `wm-presence status` output formatting.

use crate::state::DailyState;

/// Format a human-readable status line from the loaded state.
#[must_use]
pub fn format_status(state: &DailyState) -> String {
    let last = state
        .last_interaction_ts
        .as_ref()
        .map_or_else(|| "none".to_string(), |ts| ts.format("%Y-%m-%d %H:%M:%S UTC").to_string());
    format!(
        "date={} interactions={} last_interaction={} silence_emitted={}",
        state.date, state.daily_count, last, state.silence_emitted_for_window
    )
}
