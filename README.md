# wintermute-presence

`wm-presence` tells you a person you care about is okay today, without surveilling them — it reports the *fact* of an interaction with wintermute, never a word of it.

## Why it exists

Checking in on an aging parent has two bad answers. You can call constantly, which wears on both of you, or you can install a camera, which trades their dignity for your peace of mind. Both treat presence as something to be watched.

There's a third answer. If she already talks to a voice assistant during the day, the *occurrence* of that — not its content — is enough to know she's up and moving. presence listens only for that occurrence. It learns she spoke; it never learns what she said. The same idea covers the failure case: if the assistant has gone deaf and stopped hearing her, you want to know that too, before you mistake a broken microphone for a quiet day.

## What it does

A daemon on the [agorabus](https://github.com/j0yen/agorabus) local bus that watches two things and emits on a third:

- **Presence.** A wake word (`wm.audio.wake`) or a finished utterance (`wm.stt.final`) becomes one `wm.presence.summon`. From a transcript it forwards a byte count and nothing else. Go a full waking-hours window with no interaction and it emits exactly one `wm.presence.silence`, debounced, never outside the configured hours.
- **Hearing liveness.** It tracks hearing-probe results (`wm.health.hearing`) through a `Hearing → Degraded → Deaf` state machine and emits `wm.health.hearing.fail` on the edge into Deaf, `wm.health.hearing.ok` on recovery. K consecutive failures declare Deaf; K consecutive successes heal it. A missing model or unloaded detector forces Deaf immediately. This distinguishes "she was quiet" from "the ears stopped working."

Default OFF. The subject enrolls knowingly. With `enabled = false`, the daemon subscribes to nothing and emits nothing — a device that was never enrolled never reports on anyone.

## The privacy invariant

The transcript text never leaves the local bus. For `wm.stt.final`, presence extracts the byte length and discards the string — the payload type that carries it doesn't even derive `Debug`, so it can't leak into a log line. Every `wm.presence.*` message carries timestamps and counts. None carries words.

## Install

```sh
git clone https://github.com/j0yen/wintermute-presence
cd wintermute-presence
cargo build --release
install -Dm755 target/release/wm-presence ~/.local/bin/wm-presence

# Optional: run it as a user service
cp wm-presence.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now wm-presence
```

The build depends on a local `agorabus` checkout (path dependency in `Cargo.toml`).

## Run

```sh
wm-presence daemon    # the long-lived subscribe loop (what the service runs)
wm-presence status    # today's count, last interaction, hearing state
```

`status` prints one line:

```
date=2026-06-19 interactions=3 last_interaction=2026-06-19 14:02:11 UTC \
  silence_emitted=false hearing_confirmed=true hearing_liveness=hearing \
  hearing_last_ok=2026-06-19 14:02:11 UTC
```

## Configure

presence reads `<conf_dir>/presence.toml` (`--conf-dir`, default `/etc/wintermute/conf.d`). If the file is absent, it loads a disabled default and does nothing.

```toml
enabled = true

# Waking-hours window (local time, HH:MM). Silence is only ever
# declared inside this window.
waking_start = "08:00"
waking_end   = "21:00"

# Time of day at which a silent window becomes a wm.presence.silence,
# given no interaction since waking_start.
silence_threshold = "12:00"

# Hearing-liveness watcher. Set hearing_watch_enabled = false to leave
# presence behaviour untouched.
hearing_watch_enabled    = true
hearing_probe_interval_s  = 1200   # overridable via WM_PULSE_CADENCE_SECS
hearing_deaf_threshold_k  = 3      # consecutive failures → Deaf
hearing_healing_threshold_k = 2    # consecutive successes → Hearing
```

State persists across restarts via a JSON state file (`--state-path` / `WM_PRESENCE_STATE`); the bus socket is `--bus-sock` / `AGORABUS_SOCK`.

## How it's built

Rust 2024, async on tokio. The crate denies `unwrap`/`expect`/`panic` and other dishonest patterns at the lint level. Behaviour is pinned by eleven acceptance tests (`tests/acceptance_ac*.rs`) and property tests over the invariants — one summon per interaction, exactly one silence per silent window, no emission outside waking hours, no emission when disabled, and the self-topic filter that keeps the daemon from consuming its own output.

## Where it fits

Part of the wintermute voice-AI fleet, alongside [agorabus](https://github.com/j0yen/agorabus) (the local bus), the STT and TTS engines, and the other presence-side daemons. presence is the family-facing edge: everything it knows, it learns from the bus, and almost all of what crosses the bus it deliberately throws away.

## License

MIT OR Apache-2.0. Copyright Joe Yen.
