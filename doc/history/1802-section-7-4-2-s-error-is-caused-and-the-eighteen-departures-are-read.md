# 1802 — §7.4.2's error is caused, and the eighteen departures are read

Slot 5 of batch one hundred and twenty, 2026-10-27, a "clause" round; ADRs 2438 and 2439 and Q668,
none unused. I worked in `scratchpad/r1802/`. **§7.4.2 moves `departed` → `implemented`.**
**Read (ADR 2439).** `cargo run -p conformance --bin departures` names eighteen `departed` rows, the
brief's list exactly. Fourteen hold on a clause or a figure; §7.4.8, §12.6.4.6 and the JPX half of
§12.5.6.23 are the owner's (Q668, one recommendation each); §7.4.2's reason is false today;
§12.7.4.1's is half false — ADR 1198 priced a cycle detector at "a set per field" and two pointers
need none, but the bound also caps a deep acyclic chain walked once per widget, so it stands on that
half until inherited entries are resolved once per document.
**Built (ADR 2438).** "Any other characters shall cause an error": ADR 0036 skipped such a
character in silence because refusing loses the stream; the clause says what is an error and not
that an error refuses (§12.6.4.4's NOTE says "refuse" where it means it), and ADR 0343's damaged
decode already carries an error beside the bytes. `Damage::ForeignCharacter`, raised by `ascii_hex`
and the window's `AsciiHex` stage after every byte is handed over; white space is
`lexer::is_whitespace`'s Table 1. A pump reports the first damage of its chain, as
`Document::chain_over` does; `encoded_extent` still finds the `>` past the character; an object
stream keeps its last object; `Damage::stops_short` words a whole content stream's report as whole.
Consumers decide as for `CheckValue` — a substituted digit puts every later pair a nibble out of
step (`486G6C>` → `48 66 C0`): a font program is refused with its own sentence, an ICC profile takes
its `/Alternate`, the archive and serializer decline to re-encode. Three tests replace the one that
pinned the departure; the census counts the new damage.
**Premise.** The brief's list held. The hypothesis — two or three reasons false, one built — held at
one and a half. Not built: §12.7.4.1's memoised walk over six walks, a build of its own.
**Files.** `crates/pdf-syntax/src/filter.rs`, `crates/pdf-syntax/src/document/object_streams.rs`,
`crates/pdf-font/src/program.rs`, `crates/viewer-core/src/report.rs` (my hunks only),
`crates/pdf-model/examples/damaged_stream_census.rs`, `crates/pdf-model/tests/raster_golden.tsv` (one line),
`doc/conformance/ledger.toml` (§7.4.2, and
notes of §7.4.8, §12.5.6.23, §12.6.4.6, §12.7.4.1), ADRs 2438 and 2439, `doc/questions/Q668`.

**Gates.** `cargo fmt --check` and `RUSTFLAGS="-D warnings" cargo clippy --all-targets`: `pdf-syntax`,
`pdf-font` exit 0 in the worktree, `pdf-model` and `viewer-core` exit 0 in my export (siblings'
mid-edits red them here, habit 138). `cargo nextest run`: `pdf-syntax` 298, `pdf-font` 299, `pdf-model`
2259 of 2260 (the red, `outlines`' timing test at load 62, passes alone: 3 of 3), `viewer-core` 424,
`pdf-colour` 148, `pdf-transform` 706 passed. The three new tests fail, all three, with the error planted
out (trap 13). Against HEAD's six arms: 5 799 pages, 0 digests moved, each arm exit 0. `raster_golden`:
`poppler-90-0-fuzzed.pdf` p1 moved in its reports only — its `ASCIIHexDecode` content now reports
`ForeignCharacter` — regenerated, then held 974, moved 0, exit 0; `pdf-model` corpus exit 0. Rustdoc
`-D warnings` on the four crates exit 0. `--bin inapplicable`, `--bin cited`, `--bin quotations`: exit 0
each, none naming a file of mine. `cargo test -p conformance --no-fail-fast`: exit 0 after the record.
