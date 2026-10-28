# 1869 — A charstring is evaluated under an operation budget, and this crate's own walks spend the same

Slot 9 of batch 127, 2026-10-28, a "parsers" round (security, Opus 5.5), one session; ADRs 2572 and 2573, Q735 unused.
I worked in `scratchpad/r1869/` and one export of this tree built there. No ledger row moved: the slot is principle 3's.

**Sized first.** A bare CFF of ten global subroutines, each calling the next *k* times, through `read-fonts` 0.42.1's
evaluator alone under the bound: 0.65 ms at *k* = 3, 12 ms at 4, 75–117 ms at 5, 4.4 s at 8, `getrusage`'s high-water
12 MB throughout — a time bomb, not a memory one, and a 65 535-byte charstring allows *k* in the thousands.

**The premise that did not hold.** "`read-fonts` runs with none": 0.42.1 does, but 0.44.0 (under `skrifa` 0.47.0)
counts every operand and operator of a glyph's evaluation and stops it past 1 000 000. So no pre-scan was written
(ADR 2572 section 3): `skrifa` and `read-fonts` move to 0.47.0 and 0.44.0, whose one API change (an INDEX's `get` and a
charset's `glyph_id` return `Option`) is eight sites in `pdf-font` and `pdf-transform`, and every evaluator call site —
`cff::draw`, `cff::advances`, `type1`'s `draw` and `advance`, `skrifa`'s outline of an `sfnt`'s `CFF `, `accented`,
`restate` — is bounded by it. The same probe at 0.44.0: *k* = 5 refused in 17 ms, *k* = 8 in 8 ms, 12 MB. This tree's
own two walks had the same fan-out and no bound: `cff::calls_local_subr` and `embed::reach::Reached::walk` now spend
`cff::CHARSTRING_OPERATIONS`, the same count, and give up on their safe side. The upstream issue (ADR 2573) is that
the stop is reported as `CharstringNestingDepthLimitExceeded`.

**Plant.** `reading(CHARSTRING_OPERATIONS)` draws its line and one operation more is refused (`cff_budget_tests.rs`);
removing the walk's budget line turned `a_fan_out_past_the_operation_budget_keeps_every_subroutine` red (`Followed`
against `Unfollowable`), and restoring it green. HEAD's evaluator draws the *k* = 5 fixture (the probe above).

**Unfinished.** The refusal is not yet reported by name: `loading/drawing.rs` answers `None` for any evaluator error and
`show.rs` counts the glyph as one its program describes empty. Part 2 would record the error per glyph in the outline
cache and ask it where `glyph_lost_to_repair` is asked.

**Found, not fixed.** `loading/drawing.rs:95`–`97` swallow every outline error with `.ok()?`, not only the budget's.

**Gates.** In the export: `cargo test -p pdf-font` 311 lib passed, exit 0; clippy `-D warnings` on `pdf-font` and
`pdf-transform --all-targets` exit 0; `cargo fmt --check` on both exit 0; the six arms exit 0 each, 0 pages moved
against HEAD's six arms (968, 965, 968, 965, 968, 965 pages); `raster_golden --ignored` 2 passed, exit 0; the `embed`
fuzz target 600 s without a sanitizer, 2 476 586 runs, exit 0, peak 1.91 GiB. In the worktree after the three-way
merge: `cargo check --workspace --tests` exit 0; `--bin cited` exit 0; `--bin quotations` exit 0; `cargo deny check`
exit 0; `cargo test -p conformance --no-fail-fast` 68 binaries ok and `adr_status` red on ADR 2573's `proposed`, mine,
then `--test adr_status --test fuzz_workspace` 6 passed, exit 0, after `fuzz/Cargo.lock` took the three pins.
