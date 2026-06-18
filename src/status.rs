//! `wm-presence status` output formatting.

use crate::state::DailyState;

/// Format a human-readable status line from the loaded state.
#[must_use]
pub fn format_status(state: &DailyState) -> String {
    let last = state
        .last_interaction_ts
        .as_ref()
        .map_or_else(|| "none".to_string(), |ts| ts.format("%Y-%m-%d %H:%M:%S UTC").to_string());
    let last_ok = state
        .hearing_last_ok_ts
        .as_ref()
        .map_or_else(|| "none".to_string(), |ts| ts.format("%Y-%m-%d %H:%M:%S UTC").to_string());
    format!(
        "date={} interactions={} last_interaction={} silence_emitted={} \
         hearing_confirmed={} hearing_liveness={} hearing_last_ok={}",
        state.date,
        state.daily_count,
        last,
        state.silence_emitted_for_window,
        state.hearing_confirmed_in_window,
        state.hearing_liveness.as_str(),
        last_ok,
    )
}
