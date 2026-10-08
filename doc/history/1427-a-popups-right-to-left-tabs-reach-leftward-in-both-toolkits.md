# 1427 — A popup's right-to-left tabs reach leftward in both toolkits

Host slot of batch sixty-seven, 2026-10-08. ADR 1690 (no decision for 1691). No ledger row moved
(the popup's rows are not this slot's). No question written.

**Premise.** Held: `pango_tabs` handed a right-to-left paragraph's stops from the left margin, and
`lockdown.rs` called `Profile::Decoder` "the narrower of the two" beside three profiles. Not as
stated: Qt's placement, unmeasured, is Pango's — a stop from the line's right edge (300 of 400 px,
from the right margin where one is set) — so Qt needed the same per-width handing, not a label
change; and both toolkits misplace a decimal stop's number in such a line (full stop at 306 px in
Pango, 304 in Qt, against a stop at 300), so that case stays said.

**Built.** `viewer_host::popup::from_start_edge` hands each stop as its distance from the line's
start edge, nearest first, dropping those at or beyond it. `quorra-gtk` sets a right-to-left
paragraph's label as an overlay's main child under a drawing area whose `resize` reports the width;
`pango_tabs` renames a side by Pango's start-edge convention; the label is re-measured once from an
idle (a resize queued inside the allocation was measured dropped; the reversed overlay measured a
35 px label in an 18 px overlay). `RichNoteView` keeps each right-to-left block's stops and sets them
in `resizeEvent`. "a tab in a right-to-left paragraph" became "a decimal tab in a right-to-left
paragraph". Drive step 59 in the three windows. Leaders stay said: `PangoTabArray` and
`QTextOption::Tab` hold no fill (headers checked); ADR 1690 §4 names the composition that would draw
one. The `lockdown.rs` sentence names `Profile::Script` as narrower still. Taken from slot 6: the
17 round-number comment lines in `viewer-ui/tests/launch_path.rs` and 1 in
`viewer-confined/tests/confined.rs` rewritten to ADRs 0870, 0910, 0911, 0916, 0917, and both
removed from `round_numbers.rs`'s held list.

**Gates.** `rustfmt --check --edition 2024` on the 7 touched `.rs` files: exit 0.
`RUSTFLAGS="-D warnings" cargo clippy --all-targets` on `viewer-host`, `-gtk`, `-qt`, `-ui`, `-ffi`,
`-confined`, `-core`, `pdf-sandbox`: exit 0 (one run stopped at slot 3's mid-edit `raster-gpu`,
retried clean). `cargo nextest run` on the same eight: 949 passed. Planted (trap 13):
`from_start_edge`'s filter widened, its test failed; the old GTK path restored, step 59 read 305 px
against a band of 122 to 140; both reversed by patch. `cargo test -p conformance --test
round_numbers`: 2 passed; `cargo test -p conformance`: 408 passed, exit 0 (earlier runs failed only
slot 5's 41-line record and slot 6's mid-edit `bounded.rs`). Behind the lock, the release build and
`tools/drive-windows.sh`: exit 0 after 1181 s, 4.93 GiB peak (a first attempt stopped at slot 1's
mid-edit `pdf-script`, exit 101); 206 works, 0 wrong, 0 to look at, 6 not offered; step 59 at 129,
128 and 127 px. Looked at (trap 1): step 59 in all three. `accessibility_census` not run: no node.

**Unfinished.** A decimal stop in a right-to-left paragraph in both toolkits; a leader in both (a
composition, not an API, would draw it).
