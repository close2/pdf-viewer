# 1809 — A clip's own container says where it ends and which markers it names

Session 1809, a clause round in batch 121 (Slot 4); 2026-10-27. ADR 2452; ADR 2453 and Q675 unused.

**Contract.** §13.2.4.3's `/E` past the clip's end and missing marker, read against its container.

**Premises.** *The header read is admitted* held: the entry keeps "§13.2.2's viability computed and
reported in the clause's own terms" and excludes "decoding or presenting audio and video", and a
duration field, a frame count and a marker name decode no sample (ADR 2452; no owner's word needed).
*MP4 and WAV cover the corpus's clips* did not hold: a census of pdf.js's, `doc/corpora`'s and the
cache's documents found no media clip section at all, and 25 documents with media clip data naming
MP3, AVI, Flash, MPEG, MP4, QuickTime, Windows Media and AIFF, and no WAVE. The reader takes AVI and
AIFF besides MP4/QuickTime and WAVE (trap 8).

**Moved.** §13.2.4.3 `partial` → `implemented`: "if E specifies an offset beyond the end of the
next-level media clip, the end value shall be used instead, and there is no effect on viability",
and a marker that "is not present in the underlying media clip" is ignored. §13.2.4 → `implemented`
with it, derived; §13.2 stays `partial` on §13.2.6's rows.

**Built.** `pdf_model::media::container`: a bounded, `forbid(unsafe_code)` reader of RIFF WAVE, AVI,
AIFF and ISO base media headers with QuickTime's chapter titles, 4 096 items, a marker table refused
rather than cut past 4 096. `media.rs`: `Walk::span` replaces `begins_at_start`, `Walk::bound`
judges each `/B` and `/E`, `Walk::timeline` decodes an embedded clip only when an offset needs it,
once per stream; a URL, an outside file, an unread type and a damaged decode are each the reason.
ADR 2447's reading taken: an unrecognised Table 296 or 300 `/S` in a `BE` is non-viable. Fuzzed.

**Files.** New: `crates/pdf-model/src/media/container.rs`, `fuzz/fuzz_targets/media_container.rs`,
`fuzz/seed_media_container.py`, ADR 2452, this record. Edited: `media.rs`, `tests/media_viability.rs`
(`-U0`), `fuzz/Cargo.toml`, `fuzz/seeds.sh`, `doc/verify.md`, ledger §13.2.4.3 and §13.2.4,
`doc/todo/65`, `doc/state-of-play.md`. I worked in `scratchpad/r1809/`.

**Gates.** A sibling's `pdf-model` edit broke the shared build, so tier 1 ran in an export of HEAD
plus my hunks (habit 138): `rustfmt --check` and `cargo fmt --check -p pdf-model`, exit 0; clippy
`-D warnings --all-targets -p pdf-model`, exit 0; `cargo nextest run -p pdf-model`, exit 0, 2 285
passed, before the ADR 2447 take, after which `--lib media::` (39), `--test media_viability` (17) and
`viewer-core --test renditions` (5) each exit 0; rustdoc `-D warnings`, exit 0. Each new test fails
under its plant (trap 13), reversed by its own patch. Fuzz `media_container` 601 s, 273 059 589
runs, exit 0, no artefact, corpus 4 → 685. `--bin cited` and `--bin quotations`, exit 0. `cargo test
-p conformance --no-fail-fast`, exit 101, 467 passed: `records` (1808) and `names` (`readings.rs`),
neither mine. No dependency added, so no `cargo deny`; nothing drawn, so no arm.
