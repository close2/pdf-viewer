# Round 2162 — a rendition's WAVE clip is played here, and judged for that player

RFC 0015 section 5 continued: a media rendition's embedded RIFF WAVE clip decoded in the confined
media worker and played in the window, Table 291's `MH`/`BE` read into `Playing`, viability judged
for the player a clip reaches, and a resume control for `/A` false (ADRs 3349, 3350, 3351).

## What it did

- `pdf_media_worker::wave`: the RIFF chunk walk in the worker — `fmt ` (PCM, RFC 2361's A-law 6 and
  µ-law 7), `data`'s offset and length; `FRAME_LAYOUT`/`FRAME_PLACED` on the wire, checked by the
  host against the file; `Encoding::Little` for PCM of 16 to 32 bits; `MediaWorker::decode_wave`.
  The `media_sound` fuzz target runs the worker's `answer` as a layout request too; ten WAVE seeds.
- `pdf_model::media`: `Players` on `Processor`, `Clip::played_here`, `container::is_wave`,
  `Playing::of_play_parameters` (Table 291's MH over BE; `/V`, `/D`, `/RC`, `/A`, `/F` for audio,
  `/C` false honoured; `/C` true and unknown MH entries non-viable; BE set aside and said).
- `viewer_core`: `Command::PlaysHere`, sent by `quorra`, GTK and Qt at launch; `SoundData` is a
  `Recording` (`Samples` or `Wave`) and the parameters; a clip judged here goes to the window's
  player and is never handed on, and `/TF` asks nothing of it. `viewer-confined` carries both.
- `viewer_host::sound`: `Sounds::resume`, `PAUSED`; Space plays a held sound in all three windows.

**Premises checked against `doc/md/`.** The brief's numbers held: §13.2.2, Table 285 in §13.2.4.2,
Tables 290–292 in §13.2.5. §13.2.2's own EXAMPLE names "Table 293"/"Table 294" (screen parameters)
for `/V`, which is Table 291's; the code cites 291. Neither the 1991 RIFF specification nor RFC
2361 is held in `doc/`; both are cited, the facts paraphrased.

**Left, in order.** Table 294's screen parameters in MH for this player (a hidden window is
honourable); Table 287's `/BU` for a WAVE clip; a media clip section's `/B`/`/E` over a WAVE
clip, now that this player can seek; a WAVE header past the first 256 KiB; `WAVE_FORMAT_EXTENSIBLE`.

## Ledger and gates

**Ledger.** §13.2.2, §13.2.4.2, §13.2.5 stay `implemented`; notes, code and tests name both players.
**Gates.** One `-p` set — `pdf-media-worker pdf-model viewer-core viewer-confined quorra-sound
viewer-host viewer-ui viewer-gtk viewer-qt` (fmt and clippy also `viewer-ffi`): fmt exit 0; clippy
`-D warnings --all-targets` exit 0; `pdf-sandbox`, `pdf-media-worker`, `quorra-sound` `--bins` exit
0; nextest `--lib --bins --tests` under `xvfb-run -a`: 4399 run, 4397 passed, 30 skipped, 2 failed
— the manifest's `forms_values_spec_audit` test and `viewer-qt`'s `unsafe_position` (Q979), nothing
else; rustdoc `-D warnings --document-private-items` of five crates exit 0; `fuzz` `cargo check
--bins` exit 0; `cargo test -p conformance` exit 101 on `records` alone (this record's draft), then
`--test records` exit 0. `tools/drive-windows.sh` not run; no sound card here, so nothing heard.
