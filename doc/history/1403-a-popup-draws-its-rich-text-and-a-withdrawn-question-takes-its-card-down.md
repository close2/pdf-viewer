# 1403 — A popup draws its rich text, a withdrawn question takes its card down, and a script turns the page

Host slot of batch sixty-three, 2026-10-07. ADRs 1642, 1643. No ledger row moved here (§12.5.6.2
is slot 1406's row; the sentence that moves it is in the report). No question written.

**Premise.** Held: no host read a rich run; `question_withdrawn` had no event; `pageNum` was
read-only. Not as stated: `rich_text`'s layout is `pub(crate)` and writes a content stream, so no
host could call it. `pdf_model::popup::RichNote` is the host-facing reading; `rich_text.rs` gained
one additive `parts` re-export (this round's only hunk there), and slot 1406's new `Piece::Tab` is
handled.

**Rich popups** (ADR 1642). Table 172's `/RC` "shall be displayed in the popup window when the
annotation is opened": `Popup::rich`/`Comment::rich` carry each run's family path, size (`Measure`),
weight, posture, colour, lines and rise, and each paragraph's alignment, level and tag; `/Contents`
wins where its characters differ (ADR 1635's rule, the same function). The confined wire carries it.
`viewer_host::popup` holds the shared readings (size held to 0.5–3× the base, a family passed only
if `[A-Za-z0-9 _-]`, `not_drawn`, `html`); GTK sets Pango spans, Qt escaped `Qt::RichText` spans,
`quorra` its own run layout, naming faces it cannot set and drawing a right-to-left note plain,
said. Looked at in all three (trap 1): the red bold 20 pt run is red, bold and larger in each.

**Withdrawn question, page turn** (ADR 1643). Slot 1402 named no event; built against
`question_withdrawn` polled after every command, before `take_question`:
`Event::ScriptQuestionWithdrawn { document }`, wire event 24, C kind 24 (count 25). Each window
wakes once at `script_asks::wake_after` and sends `Tick { millis: 0 }`; `quorra` drops its card,
GTK closes its dialogue marked answered, Qt rejects its `exec`; confined says it had answered.
`PDF_VIEWER_SCRIPT_ANSWER_WAIT_MS` sets the wait. `ScriptEdit::GoTo` is taken before focus requests
and made as `go_to(.., Turn::Requested)`, at most four a command, front document only.

**Gates.** `rustfmt --check --edition 2024` on the 29 touched `.rs` files: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy --all-targets` on `pdf-model`, `viewer-core`, `-host`, `-confined`, `-ffi`,
`-gtk`, `-qt`, `-ui`: exit 0. `cargo nextest run` on the eight: 2 835 passed, once the C program's
expected kind count moved 24 → 25. Planted (trap 13): the page-turn call and the event push each
removed, both new core tests failed, restored. `cargo test -p conformance`: exit 0. Behind the
lock, `tools/drive-windows.sh`: exit 0, 180 works, 0 wrong, 0 to look at, 4 not offered (the
floor's 3 and confined's 48); steps 48, 49, 50 work wherever they can (50: 572, 701, 714 red
pixels, 0 on the `/Contents` control); 598 s held. `accessibility_census` not run: no node changed.

**Unfinished.** The C ABI hands no rich note to a caller (a header question); `quorra` lays no UAX
#9 order across runs and sets one family; letter-spacing, font scales and tab stops are said.
