# 1469 — A note is typed into in the faces its window drew, and a timer that changes nothing leaves the page

Slot 2 of batch seventy-four, 2026-10-08, a host round. ADR 1782; ADR 1783 and Q365 not used. No
ledger row moved.

**Premise.** Held for three of the four: record 1464's "Unfinished" names them, and
`run_timers` answered `usize`. **The fourth did not**: `event.keyDown` reads false not only because
no window reports an arrow-made choice. `ViewState::set_field` raises no Table 199 `/K` for
`Entered::Chosen`, which the table makes a `shall` where the user "modifies the selection in a
scrollable list box", and `quorra`'s choice list takes no arrow key. A report from the windows would
reach nothing, so (3) is not built (ADR 1782, Consequences). The brief's Qt route, typing into the
note's own `QTextDocument`, would save `/RC`'s resolved text over `/Contents`. **Hypothesis held**:
GTK's tags carry face, size and colour and no glyph scale, said under the editor as the label says it.

**Built (ADR 1782).** `viewer_host::popup::run_spans` places each run on `/Contents` bytes and
`editor_not_drawn` says what of a note's paragraphs an editor does not draw. GTK sets a tag per run
from `RunFace`, the numbers its Pango span is now written from too; Qt sets `runFormat` through a
`QSyntaxHighlighter`. Both are reset from nothing when the window's answer changes, so a typed
character that makes the window plain makes the editor plain. Qt's plain note is a `NoteView` over a
plain document, so its editor starts at the press. Every window says the caret's byte, and Qt counts
`\r\n` as one position. `run_timers` answers `ScriptsRan`; the core stales on `changed`.

**Found.** The second lane was held by slot 4's `--long` campaign, so a `--tree 6` drive queued
behind it; the drive went on the first lane as the large walk the brief names (1 658 s queued).
Driving the note steps in `quorra-confined` with `--step` puts them where the whole drive does
not: that window draws no popup, so its two `wrong` there are the step list's, not a fault.

**Unfinished.** `keyDown`: the core's `/K` at a choice selection, then `quorra`'s arrows, then the
windows' report — `pdf-model/src/view.rs` is no file of this slot.

**Gates.** rustfmt `--check` on my 7 Rust files: clean. `RUSTFLAGS="-D warnings" cargo clippy
--all-targets` on `pdf-model`, `viewer-core`, `viewer-host`, `viewer-gtk`, `viewer-qt`: exit 0 (once
101 on slot 1's mid-edit `rich_text.rs`, then 0). `cargo nextest run`: `pdf-model` 2 013 passed, 19
skipped; the four host crates 657 passed, 2 skipped; the old count rule planted back fails the new
timer test. `cargo test -p conformance`: 427 passed, 0 failed. Lock, `--tree 12`: steps 64, 67, 72
in four windows, 10 works and the two above (132 s); the whole drive on copied binaries: exit 0, 258
works, 0 wrong, 0 to look at, 6 not offered (830 s). Photographs of 64 read: the red run red in
GTK's and Qt's editors, plain after `|`. `launch_path` not run: no open path changed.
Duration 5 080 s from the brief's reading.
