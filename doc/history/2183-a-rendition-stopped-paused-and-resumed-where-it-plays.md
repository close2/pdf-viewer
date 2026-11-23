# Round 2183 — a rendition stopped, paused and resumed where it plays

§12.6.4.14's owed half: Table 218's `/OP` 1, 2, 3 and 4 on a sound a window plays. ADR 3433.

## What it did

- **The link.** Every sound a rendition action plays carries `viewer_core::Associated` (`/AN`, and
  whether `/OP` was 4) in `SoundData::rendition`, whatever its `/W`, across the confined wire; the
  player's thread keeps it on the sound in hand.
- **The operations.** `/OP` 1–3 pass §12.6.4.14's precondition and become `Event::OperateMedia`,
  in the chain's order with plays (`interact::MediaRequest`). `Player::operate`: stop removes the
  association and clears the `/W` 3 painting; pause stops the sink's stream, keeps the device, and
  folds the device's count (or the freeze so far) into the clock (`MediaClock::pause`); resume
  starts the rest from there. `/OP` 4 (`RenditionOperation::PlayOrResume`) resumes the paused
  rendition of its annotation or does nothing while it sounds. "No effect" is `Heard::Unchanged`,
  said. The thread now tells the painting each change (`Said::Painted`).
- **Windows.** GTK and Qt through `native_window`'s `Sounds::operate`, `quorra` through its player;
  `quorra-confined` and a window without its helper say a handed-on clip is beyond reach. The C ABI
  gains `QUORRA_EVENT_OPERATE_MEDIA` (32; the count 33, header and C# constants).
- **/JS beside /OP.** Unchanged (ADR 2988), and a core test holds it for a stop.
- **Tests.** `viewer-core/tests/rendition_operations.rs` (4), `viewer-host/tests/rendition_operations.rs`
  (2; a planted pause that kept no offset failed it, reversed), `MediaClock` and painting units,
  the confined round trip, and one GTK and one Qt window test over `sounding.rs`'s new rendition
  document (a WAVE clip and three links).

**Premise.** The brief's "Table 214" is Table 218 in ISO 32000-2; the text it names held.

**Ledger.** §12.6.4.14 `partial` → `implemented`; §12.6.4 and §12.6 `partial` → `implemented`
(aggregates). `doc/todo/65` no longer lists them.

**Left, in order.** A macOS check of the sound path; a `quorra` window test of the hand-on.

**Gates.** One `-p` set, `viewer-core viewer-host viewer-qt viewer-gtk viewer-ui pdf-model
viewer-confined viewer-ffi`: fmt `--check` exit 0; clippy `-D warnings --all-targets` exit 0;
`pdf-sandbox` and `pdf-media-worker --bins` exit 0; nextest `--lib --bins --tests` under
`xvfb-run -a`: 4 501 run, 4 499 passed, 2 failed — the manifest's `type3_widths_range` entry and
`viewer-ffi`'s kind count, fixed (33) with the header, the C# constants and the C program's
expected line, then `viewer-ffi` alone exit 0; rustdoc `-D warnings --document-private-items`
exit 0; `fuzz` `check --bins` exit 0; `cargo test -p conformance` exit 0 after the §13.2.6.1 row
named the painting's renamed test.
