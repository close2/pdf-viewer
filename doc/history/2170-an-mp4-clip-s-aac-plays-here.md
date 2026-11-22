# Round 2170 — an MP4 clip's AAC is decoded in the confined media worker and plays here

RFC 0015 section 5's step 9 (A946): ADRs 3381–3384.

## What it did

- **The wire (ADR 3381).** A clip crosses box by box, its movie whole, its access units in runs:
  `FRAME_BOX` places each top-level box from 32 bytes, `FRAME_MOVIE` (8 MiB at most, the one request
  past `MAX_RUN_BYTES`) answers the unit table and the edit list's figures, `FRAME_UNITS` runs of 32
  go to an `aac::Session` the worker holds for the clip. The host checks every answer against the
  file's length and the request; `Profile::Media`'s 64 MiB ceiling is unchanged.
- **The decoder (ADR 3382).** `rusty_aac` =2.1.1, `default-features = false`: Apache-2.0, no
  dependency, no C, `cargo deny` exit 0, three `unsafe` left in the scalar build (fresh-capacity
  fills). Raw units into `decode::Decoder`, never ADTS; a refused unit is silence, counted and said.
- **The edit list (ADR 3383).** `mvhd`, `edts`/`elst` read; leading empty edits become silence, one
  edit at rate 1 skips the priming and keeps its duration; any other list plays whole and is said.
- **Played here (ADR 3384).** `container::is_played_here`: WAVE and the `audio/` MP4 types;
  `Recording::Mp4`, the confined protocol's sound kind 3, `viewer_host::sound` → `decode_mp4`.
- **Conformance walk** (`tests/aac_conformance.rs`, ignored, not a gate): all 22 held streams decode;
  of the 22 (each referenced, two HE-AAC ones through `add-opt/` candidates), 18 within one 16-bit LSB (al00sf and the 48-channel al08 among them; the HE-AAC
  pair against the high-quality references); OFF: al03 (as RFC 0012), al07 in four of six channels,
  al18/al19 (PNS). The suite states no tolerance and 14496-4/-26 are not held: the bar is RFC 0012's.
- **Tests**: `tests/aac.rs` (a tone encoded by `rusty_aac`, laid out in boxes, decoded through the
  worker: priming trimmed, delay, list not applied, bound, refusals, `presentation`, `top_level_box`),
  wire refusals, a viewer-core rendition test. **Fuzz**: `media_aac`, seeded by
  `fuzz/seed_media_aac.py` from the held streams.

**Left, in order.** al07's four channels and al03 read against 14496-3 and reported upstream; the
standard's own tolerance once 14496-4/-26 are held; Q988's text check now covers `elst` too.

**Gates.** One `-p` set, `pdf-media-worker pdf-model viewer-core viewer-host viewer-qt viewer-confined`:
fmt exit 0; clippy `-D warnings --all-targets` exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib
--bins --tests` under `xvfb-run -a`: 4 232 run, 4 231 passed, one failure (this round's own unit
test, fixed; the worker's 62 then passed twice), the audit manifest's difference otherwise empty;
rustdoc `-D warnings --document-private-items` exit 0; `cargo deny check` exit 0; `fuzz` `cargo
check --bins` exit 0, clippy of `media_aac` exit 0; `tools/fuzz.sh media_aac` 600 s, 9 515 runs,
no artifact, ft 7 111 → 20 516; `cargo test -p conformance` exit 101 on `seeds.sh`'s missing arm and
this record (both fixed), then the whole suite exit 0.

**Ledger.** No row moves: §13.2.1 stays `out-of-scope` until video is opened; §13.3 is unchanged.
