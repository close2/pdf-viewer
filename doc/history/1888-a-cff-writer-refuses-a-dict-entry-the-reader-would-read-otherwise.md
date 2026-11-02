# 1888 — A CFF writer refuses a DICT entry the reader would read otherwise, and states a width only where it is read

Slot 8 of batch 129, 2026-10-29 to 2026-11-02, Opus 5.5, four sessions (the first and third handed over on
context, the second stopped at the weekly limit), a parsers round (robustness); ADR 2610 used, ADR 2611
and Q764 unused. Ledger: no row moved — the program's inside is TN 5176's, not ISO 32000-2's.

**Premise.** The brief's hypothesis held: one operand reader, `exactly::<N>`, with one rule — an offset
reads only where stated once with its table's count of integer operands — used by every writer, and a
program stated otherwise is refused by name on every path (`misstated_entry`). What did not hold: the
operand rule was not the whole disagreement — the widened fuzz target's read-back property stopped 1 360
times in ten minutes, and it was right each time. The writer was wrong six ways, each now refused or read
as the reader reads — a Private DICT holding a reserved byte; a width inserted before an `rmoveto` of one
operand; an offset into the header, Name or Top DICT INDEX or past the end; an unreadable `FDArray`
rewritten as empty; and a real holding the nibble 0xd, which now ends where `read-fonts` ends it
(`real_width`, shared by both DICT scanners), so the `nominalWidthX` after it is no longer hidden. The
census decided the rule: without the pack, after each fix, its 424 per-document lines are identical to
HEAD's, so the refusal stands (ADR 2610).

**Plant.** The brief's Private DICT entry of five operands through `with_advances`, and each of the six
fixes planted away in turn: its test fails each time and passes restored. The `getrusage` probe of a
thousand overlapping Private DICTs: 0.005–0.007 s and 11.8–12.6 MiB peak resident, against 35.5–49.2 s and
36.9–37.3 MiB planted.

**Gates.** `cargo nextest run -p pdf-font` exit 0, 336 of 336; `cargo clippy --no-deps -p pdf-font
--all-targets -- -D warnings` exit 0; `rustfmt --edition 2024 --check` on the two files exit 0; under
`fuzz/Cargo.toml`, `cargo fmt --check` exit 0 and `RUSTFLAGS=-D warnings cargo clippy --all-targets` exit
0; `raster_golden` exit 0, 2 passed; the six arms (session 1, before the writer refusals) exit 0,
identical by page against HEAD's (968 pages at 1x, 965 at 4x, cpu, gpu and compute); the census without
the pack exit 0, 424 per-document lines identical against HEAD's; 1 709 artifacts replayed, 1 704 clean, 5
attributed below; `cff_write` ten minutes fork mode under the bound, exit 0, 1 386 836 executions, 2 462
edges, 0.11 GiB peak, 5 stops attributed below; `--bin cited` exit 0; `--bin quotations` exit 0; `cargo
test -p conformance --no-fail-fast` exit 0, 470 passed in 69 binaries, none failed.

**Found, not fixed.** Offsets the rewrite shifts and should refuse, beside `offset_outside` in
`crates/pdf-font/src/cff.rs`: a Top DICT `Private` of negative size (`top_dict`, `OP_PRIVATE`, ~832; the
Font DICT arm ~866 alike); a `Private` inside the String INDEX, shifted by the whole growth in
`Layout::with_glyph_tables` (~749–800). Charstring-level, not this slot's: a `callsubr` or `seac` on an
empty stack, which `read-fonts` answers from a stale slot (`fuzz/fuzz_targets/cff_write.rs:91`, five
artifacts); three of round 1882's artifacts at `cff_write.rs:185` (`with_cids_given`'s read-back); and
`with_advances` reads each Font DICT's Private DICT twice (`nominal_widths`, `local_subroutines`).
