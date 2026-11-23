# Round 2174 — a sound's section, its hidden window, and a WAVE file walked a window at a time

Clause 13 sound's leftovers (A946, RFC 0015 section 5): ADRs 3397–3400.

## What it did

- **WAVE (ADR 3397).** `pdf_media_worker::wave::layout` reads one window from an offset and answers
  `FRAME_PLACED` or `FRAME_ONWARD` (the next chunk header, the format so far); the host checks each
  step forward inside the file, at most `MAX_WAVE_WINDOWS`. `WAVE_FORMAT_EXTENSIBLE` is read by its
  `SubFormat` GUID (PCM, IEEE float, A-law, µ-law over the base GUID); 32-bit IEEE float is
  `Encoding::Float`; the channel mask is carried and said where it is not the first speakers.
- **Tables 287–289 (ADR 3398).** The brief's "Table 287 `/B` `/E`" are Table 289's (a media clip
  section's offsets); Table 287 holds only `/BU`; a sound object's `/B`, `/E`, `/C` are Table 305's
  and were already decoded. A chain of sections ending in a played-here clip is played here: time
  offsets and markers placed, an empty range valid, frame offsets non-viable in `MH` (or under an
  `MH` above) and said in `BE`. `Playing::section` crosses `viewer-confined`; `viewer_host::sound`
  cuts the frames. `/BU` is honoured by this player, which resolves no URL.
- **Table 294 (ADR 3399).** `/W` 2 (hidden) is this player's, `/B` `/O` `/M` `/F` ignored as the
  table says; any other `/W` non-viable in `MH`, said in `BE`.
- **al07/al03 (ADR 3400).** al07 decodes within 0.004 LSB in all six channels: the walk chose its
  channel and lag over leading silence; it now chooses where the reference carries signal. al03's
  23 differing blocks are its out-of-order window sequences, `rusty_aac`'s `imdct_and_window`;
  `doc/upstream/rusty-aac-al03-window-sequence.md` is the unfiled issue. 14496-3 is not held.
- **Tests**: wave units, an extensible float file behind 300 KiB of `LIST` end to end, two
  `media_viability` tests, `sectioned`; fuzz `media_sound` reads both replies, four new seeds.

**Left, in order.** A marker-offset test over a WAVE with cue points played here; the channel mask
applied by speaker rather than said; al03 filed upstream by a person; Table 294 `/W` 3's
background painted in the annotation rectangle while a sound plays.

**Ledger.** §13.2.4.2, §13.2.4.3, §13.2.6.1 stay `implemented`; notes and tests name this player.

**Gates.** One `-p` set, `pdf-media-worker pdf-model viewer-core viewer-host viewer-qt
viewer-confined`: fmt exit 0; clippy `-D warnings --all-targets` exit 0 (after three fixes of this
round's own code); `pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` under `xvfb-run -a`:
4 249 run, 4 244 passed, 5 failed — the manifest's five clause 11 audit tests, difference empty;
rustdoc `-D warnings --document-private-items` of four crates exit 0; `fuzz` `cargo check --bins`
exit 0, clippy exit 0; AAC conformance walk (ignored) exit 0; `cargo test -p conformance` exit 101
on `records` alone (this record's gates), then `--test records` exit 0.
