# AUTOBUILDER_PROGRAM — wintermute-presence

## Role

You are the **edit-agent** for `wintermute-presence`. You iterate on `src/`
to make all MUST-level acceptance tests pass under the constraints in
`agent/intent-card.json`.

## Critical privacy invariant

**NEVER write transcript text to any output.** The daemon receives
`wm.stt.final` payloads that contain a `transcript` field. You must only
extract `transcript.len()` — a character count. The word "len" or the
count integer may appear anywhere. The actual transcript string must never
appear in:
- Any published bus message
- Any log line (tracing macros or eprintln)
- The state file
- Any test assertion that might print it on failure

This is the load-bearing privacy constraint. Violating it fails AC2.

## Ownership boundary

- You edit ONLY files under `src/` and `Cargo.toml` (dependency additions only).
- Tests under `tests/acceptance_*.rs` are read-only harness files. If a test
  appears wrong, write `agent/intent_card_amendment_request.json` and stop.
- `scripts/`, `agent/*.toml`, `agent/intent-card.json`, `clippy.toml`,
  `deny.toml`, `rust-toolchain.toml` are all read-only.

## Hard constraints

- Rust edition 2024, MSRV 1.85
- `deny(unsafe_code)` — no unsafe blocks, no `mem::transmute`, no raw pointers
- No `unwrap()`, `expect()`, `panic!`, `todo!()`, `unimplemented!()` in src/
- All `Result` must be propagated, not swallowed
- `clippy -- -D warnings` must pass
- `cargo deny check bans licenses sources` must pass (use `bans licenses sources`,
  not full `cargo deny check` which may break on CVSS 4.0 advisories)

## Architecture sketch

```
src/
├── lib.rs         — re-exports; public API surface
├── main.rs        — clap CLI entrypoint (daemon / status subcommands)
├── config.rs      — load /etc/wintermute/conf.d/ presence config; opt-in gate
├── state.rs       — daily state file (JSON, ~/.local/state/wintermute-presence/state.json)
├── daemon.rs      — subscribe loop; routes wm.audio.wake + wm.stt.final
├── silence.rs     — waking-hours window check; silence detector (debounced)
└── events.rs      — topic constants and payload types (Serialize only; no transcript text)
```

## Topic routing

- Subscribe prefix: `wm.audio` (catches `wm.audio.wake`) + `wm.stt` (catches `wm.stt.final`)
- OR subscribe prefix `wm.` and filter in-process (simpler, slightly noisier)
- Self-filter: skip any event where `event.from == own_session_id`
- Self-filter: skip any event where `event.topic.starts_with("wm.presence.")`

## State file shape

```json
{
  "date": "2026-05-30",
  "daily_count": 3,
  "last_interaction_ts": "2026-05-30T14:23:00Z",
  "silence_emitted_for_window": false
}
```

Atomically write to `.tmp` then `rename` to final path so a crash mid-write
never corrupts the file.

## Silence detection logic

```
fn should_emit_silence(now: DateTime<Local>, cfg: &Config, state: &State) -> bool {
    // 1. Are we inside the waking-hours window?
    let window_start = cfg.waking_start (e.g. 08:00 local)
    let threshold_time = cfg.silence_threshold (e.g. 12:00 local — "by noon, no interaction")
    if now.time() < window_start || now.time() < threshold_time { return false; }
    // 2. Outside waking hours entirely?
    if now.time() >= cfg.waking_end { return false; }
    // 3. Any interaction today?
    if state.daily_count > 0 { return false; }
    // 4. Already emitted for this window?
    if state.silence_emitted_for_window { return false; }
    true
}
```

## Iteration strategy

1. Iter 0: stub all modules, make `cargo check` pass.
2. Iter 1: implement `state.rs` (round-trip test AC3 passes).
3. Iter 2: implement `config.rs` + opt-in gate (AC7 passes).
4. Iter 3: implement `events.rs` + `daemon.rs` subscribe loop with mock bus
   (AC1, AC2, AC9 pass via mock).
5. Iter 4: implement `silence.rs` (AC4, AC5, AC6 pass).
6. Iter 5: implement `status` subcommand (AC8 passes).
7. Iter 6: AC10 (systemd unit check) + AC11 (clippy clean).
