# Round 2185 — audio's clock and sinks, measured in use

A946's condition for video: audio shipped (rounds 2148–2183) and measured in use. ADR 3441.

## What it did

- **The instrument.** `crates/viewer-host/tests/audio_in_use.rs` (ignored, `// not a gate:`) drives a
  window's `Player`, the confined worker and the sound helper; `tools/audio-in-use.sh` builds both
  programs in the `gates` profile, makes the AAC LC clip with `ffmpeg`, fetches ISO/IEC 14496-26's
  two HE-AAC streams pinned by SHA-256, and runs it. `rustix` (`fs`, `pipe`) is a dev-dependency of
  `viewer-host` for the pipe. Listed in `doc/verify.md`.
- **The device.** No card the agent user may open (not in `audio`; the RX 580's HDMI has no
  monitor; no sound server); `null` alone has no clock. So the helper's `HOME` names an `.asoundrc`
  making the default a `plug` → `file` → `null` into a one-page pipe read at 48 kHz on the monotonic
  clock. The sounds carry their frame index, so resume points and lost frames are read off the
  frames. A card's buffer and a crystal's drift stay unmeasured, said in the review.
- **The readings** (four runs, load averages 10 to 358 on 32 processors):
  `doc/reviews/2185-audio-measured-in-use.md`, and RFC 0015's new section 9.1 as command and dated
  readings. The clock holds (single-digit ppm, ±30 ms, no frame lost in a pass); the worker is far
  inside 64 MiB and its request budget; a sound's start grows with the sound (whole-sound decode);
  a resume repeats 10–40 ms; the helper holds two passes at once (120 MiB for 160 s) with 60 ms
  to 2.6 s of silence between them; underruns follow the load (8–56 a minute at load 10–21,
  300–457 at 91–262), with no paging.
- **Q1006** asks whether to open video (section 6's fourteen rounds): two audio rounds first
  (streaming to the helper; a priority and a buffer for its stream), a real card when there is one.

**Ledger.** No row touched: 883 rows before and after (`tools/state.sh --round 2185 ledger`).

**Launch.** `tools/state.sh --round 2185 launch` before (exit 101: one figure below its band,
`opt_demo.pdf` `peak_anon_mib` 2.891 under 3.500, on the unchanged tree) and after (exit 0, none
outside); the same frames; no code a window runs was changed.

**Left, in order.** Q1006's answer; the two audio rounds if chosen; the reading on a real card.

**Gates.** One `-p` set, `viewer-host pdf-media-worker quorra-sound viewer-qt`: fmt `--check`
exit 0; clippy `-D warnings --all-targets` exit 0; `pdf-sandbox --bins` and the two programs'
`--bins` exit 0; nextest `--lib --bins --tests` under `xvfb-run -a`: 552 run, 552 passed, 5
skipped, against an empty manifest (`audit-diff.py`: failed 0, expected 0); rustdoc `-D warnings
--document-private-items` of `viewer-host` exit 0; `cargo test -p conformance` exit 0 once this
record stated its gates. No public type changed, so no `fuzz` check.
