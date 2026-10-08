# 1464 — A rich note's caret stands at its glyphs, and a script that changes nothing leaves the page

Slot 3 of batch seventy-three, 2026-10-08, a host round. ADRs 1770, 1771; no row moved, no question.

**Premise.** Held: record 1458's "Unfinished" names the rich caret, ADR 1752 section 3 the stale
page; `rich::draw` in `chrome/rich.rs`, one `run_annotation_scripts` use in `interact.rs` (two more in
`viewer.rs`). §12.5.6.6 states no white-space collapse: it is chapter 27's and `same_characters`'
(ADR 1635), §12.5.6.2 NOTE 1's "textually equivalent" the reason the alignment is exact.
**Hypothesis did not hold**: Pango (`xy_to_index`) and `QTextDocument` (`hitTest`) report a rich
run's glyph offset; the toolkits' limit is that they retype in a plain view replacing the window.

**Built (ADR 1770).** `viewer_host::popup::rich_offsets` aligns each rich paragraph's bytes to
`/Contents`; `rich::lay_out` draws and places from one layout, and `quorra`'s caret, press and
arrows use it. GTK's and Qt's editors start at the character pressed (Qt's plain `QLabel`: the end).
Drive step 64 presses the red run, moves Right, Left, Left: `A |red word.` in all three.

**Built (ADR 1771), with slot 2's handback.** `viewer-core` stales on `ScriptsRan::changed` and
leaves refusals to `handed` (slot 2's `.handed` hunks replaced); step 68 counts 0 renders after a
logging `/E`, 5 after the writing one. `Command::Keys` (wire kind 41) is sent by every window;
`Event::Console` (event kind 27, `QUORRA_EVENT_KIND_COUNT` 28, greeting `PDFVCF13`) is said by name.
Step 71: console shown, cleared, hidden said; Shift and Control read true, then false.

**Found.** `quorra` traces no pointer message by name, so `needs render` counted 0 either way: the
count is `render ready`, with the writing `/E` as control (trap 13). Editing the drive script while
it ran broke the run at line 3152 — bash reads a script as it goes. winit hears a modifier only
with the keyboard, so step 71 focuses the window first.

**Unfinished.** Toolkit editors show the note plain while typed into; Qt's plain note starts at the
end; `run_timers` answers a count; no window reports an arrow-made choice (`keyDown` false).

**Gates.** rustfmt `--check` on my 24 Rust files: clean. `RUSTFLAGS="-D warnings" cargo clippy
--all-targets` on `viewer-core`, `viewer-host`, `viewer-confined`, `viewer-ffi`, `viewer-ui`,
`viewer-gtk`, `viewer-qt`: exit 0. `cargo nextest run` on those: 946 passed, 0 failed; planted
defects fail the RTL caret, stale and keys tests. `cargo test -p conformance`: 426 passed, 0
failed. `cargo check` of `fuzz/`: exit 0 (6 s). Lock, `--tree 6`: steps
64, 67, 68–71 in four windows, 20 works before the step 71 fixes, then 71 works (230 s, 3 s).
`--tree 12`: the whole drive exit 0, 254 works, 0 wrong, 0 to look at, 6 not offered (812 s), on
copied binaries. `launch_path` not run: no open path changed. Duration 6 169 s.
