# 1625 — A replacement is spoken once, and a C caller's copy is the copy

Slot 3 of batch ninety-eight, 2026-10-20, a host round: ADRs 2086 and 2087, no question. Round
1621's three "Found, not fixed" defects, each fixed with a test red on HEAD plus the tests alone.

**Premise.** Held: all three as 1621 wrote them. **Hypothesis.** (1) did not hold: `push_replacement`
is right (ADR 1515's shared span), and `device_lines` now speaks a code whose span the line's last
character spoke as that character, its box grown over the glyph (ADR 2086).
(2) held in substance, not in shape: `copy_text` sending `Command::Copy` would ask twice across C's
two-call idiom and drop an `Asking`; the session keeps what `Event::Copied` released, through
`viewer_host::copied`, cleared at each `Command::Copy` (ADR 2087). (3) held: a call.

**Red, then green.** `a_replacement_over_two_codes_is_one_character_of_its_line`: `Der Drucc`, then
`Der Druc`, 8 characters, the last 26.7 px wide over `u`'s 17.8. `the_copy_text_is_what_the_copy_
released_at_the_readers_level`: `Seiten\u{ad}\nzahl.` with no copy asked, then `NoAnswer` there, at
`on` with bit 5 clear, while `ask` waits and after a `no`, and `Seiten\u{ad}zahl.` after a grant.
`a_copied_event_answers_as_the_hosts_one_decision_does`: `Ok(("", false))` for a copy of nothing,
then `NoAnswer`. The C program: `there is nothing to answer with` before `quorra_copy`, then `448
byte(s) in logical content order, the event's own text: 1`, both asserted.

**Walk.** Stage 5's sentence on `copy_text`, the copy diagram's `FFI` edge (now from `CHOOSE`) and
the caret paragraph changed because the bugs are fixed; no string the walk quotes moved.

**Gates.** In an export of HEAD plus this round's `-U0` hunks (siblings' `viewer-ffi` and
`pdf-colour` edits did not compile mid-batch): `rustfmt --check` on 7 files, `cargo fmt --check -p
viewer-core -p viewer-ffi`, `RUSTFLAGS=-D warnings cargo clippy --all-targets` on both: exit 0;
`cargo nextest run -p viewer-core -p viewer-ffi`: 455 passed, 2 skipped, exit 0 (the C program and
`header_and_interop_agree` among them); red tree: the four tests failed, exit 100. In the worktree:
`cargo test -p conformance`: 59 binaries, 0 failed, exit 0; `--test teaching`: 3 passed.
`tools/state.sh hosts`: exit 0, Query 35 of 36 — `LogicalSelection` is no longer asked by the ABI,
by decision (ADR 2087 §3; the windows reading's line restated). `tools/drive-windows.sh --step drive
--step accessibility` over `quorra`, `quorra-gtk`, `quorra-qt` on the export's release binaries:
119 works, 0 wrong, 3 not offered (01 in GTK, 27 in both toolkits), exit 0; `15-restrictions` and
`26-accessibility` work in all three; the confined window has no copy gesture.
Rustdoc on both crates fails on HEAD's own `[t]`-style quotes, none on this round's lines.

**Files.** `viewer-core`'s `viewer/geometry.rs`, `accessibility.rs` (a doc line) and
`tests/tagged_copy.rs`; `viewer-ffi`'s `src/{session,events,abi}.rs`, `include/quorra.h`,
`c/open_a_page.c`, `tests/a_c_program_drives_the_abi.rs`; the walk, `doc/todo/30` (one row),
`tools/state.sh` (one reading line), ADRs 2086, 2087, this record. 6804 s.
