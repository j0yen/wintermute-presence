# wintermute-presence

> Privacy-first presence heartbeat daemon: knows she's okay without watching her.

`wm-presence` is an [agorabus](https://github.com/j0yen/agorabus) daemon that
lets you know Mom is okay — without surveilling her. It listens only to the
*fact* that she interacted with wintermute (never the content), and emits two
signals:

- **`wm.presence.summon`** — she talked to it (character count only, no text).
- **`wm.presence.silence`** — no interaction in the configured waking-hours
  window. One signal per window, debounced, never outside waking hours.

Default OFF. She enrolls knowingly (via `wm-family enroll --presence`).

## Acceptance tests

1. A published `wm.audio.wake` produces one `wm.presence.summon` on the bus.
2. A `wm.stt.final { transcript }` produces `wm.presence.summon { transcript_len }` — count only, no text.
3. The daily counter survives a daemon restart (state-file round-trip).
4. Zero interactions in the configured window → exactly one `wm.presence.silence` emitted.
5. An interaction in the window suppresses the silence emission for that window.
6. Outside waking hours, no `wm.presence.silence` is ever emitted.
7. With presence **disabled**, the daemon subscribes and emits nothing.
8. `wm-presence status` prints today's count and last-interaction timestamp.
9. The daemon applies a self-emitted-topic filter and does not consume its own `wm.presence.*`.
10. systemd unit installs at the consistent bin path (no cargo-bin drift).
11. `cargo test` green; `cargo clippy` clean.

## Install

```sh
git clone https://github.com/j0yen/wintermute-presence
cd wintermute-presence
cargo build --release
install -Dm755 target/release/wm-presence ~/.local/bin/wm-presence

# Optional: install systemd unit
cp wm-presence.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now wm-presence
```

## Configuration

Place a TOML config at `/etc/wintermute/conf.d/50-presence.toml` (or
`~/.config/wintermute/presence.toml`):

```toml
enabled = true
waking_hours_start = "08:00"
waking_hours_end   = "21:00"
silence_threshold_minutes = 60
state_path = "~/.local/state/wintermute-presence/state.json"
```

With `enabled = false` (the default), the daemon subscribes to nothing and
emits nothing. A device without enrollment never phones home about Mom.

## Privacy guarantee

presence reads **only** the occurrence of an interaction and, for
`wm.stt.final`, its transcript length as a character count. The transcript text
itself never leaves the local bus. `wm.presence.*` carries timestamps and
counts — no words.

## License

MIT OR Apache-2.0. Copyright Joe Yen.
