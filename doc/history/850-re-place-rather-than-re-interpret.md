# 850 — Re-place rather than re-interpret

A loop round on `doc/todo/46`, taking the seam ADR 0775 chose and left unbuilt. A fifth round, so
`doc/todo/02` §2 ran whole and §5 rebuilt and installed the binaries.

## What was decided, and the measurement that decided it

`doc/todo/46` said to measure the `DisplayList` clone **before** designing around it: tens of
microseconds means a contract-preserving round, anything else means a transform node in
`pdf_render::DisplayList` and a three-rasteriser change with its own ADR. Measured first, with a new
`pdf-model/examples/replace_cost`: 7 to 39 µs against interpretations of 0.73 to 12.7 ms, on the
five ISO 32000-2 pages ADR 0775 named. So the list is rebuilt by copying and no rasteriser was
touched.

The construction is the one `doc/todo/46` rated harder — restore an `Interpreter` from an owned
snapshot — taken because the objection to it ("about forty fields, and a field forgotten is a report
silently lost") is answerable by the compiler: `Interpreter::checkpoint` destructures the
interpreter exhaustively with no `..`, and `restore` does the same to the `Checkpoint`, so a field
added to either fails to build until somebody has decided what it is. The alternative — merge two
interpretations — was rejected on a reading: the per-font "this font drew nothing" report is over
the whole page, and a tail-only interpretation would raise it for a font the content half drew with.

ADR 0777 has the argument, the tables and the three questions answered.

## What it bought

Before and after in one sitting, the "before" built from the same tree with the change reversed by
patch and both crates' `lib.rs` touched (trap 10b), `viewer-core/examples/zoom_cost` on the
specification's own PDF:

| | before | after |
|---|---|---|
| worst notch over all 341 pages | 5.040 ms (page 407) | **1.035 ms** (page 962) |
| page 407 | 4.993 ms | 402 µs |
| page 1001 | 3.809 ms | 529 µs |
| page 504 | 3.626 ms | 278 µs |
| page 10 | 566 µs | 96 µs |

## The finding

The seam's own condition — asked *before* the annotation pass, to decide whether the state is worth
keeping — read Table 167's bit out of `/F`, which is not what decides it. §12.5.6.4 sets `NoZoom` on
a `Text` annotation whatever the file says and §12.5.6.10's four markup subtypes have it cleared, so
the new condition named a different population from the one `annotation::decide` acts on.
`pdf-model/examples/replacement_census`, written to check the seam over a corpus rather than over
eight pages, printed the five pdf.js documents where the two disagreed on its first run — every one
a `Text` annotation with no flag set. Those pages would have re-interpreted whole on every notch,
correctly and silently, with nothing able to say so. `annotation::view_flags` is the one reading now
and the census reports zero.

The same reading corrects a number this project had been quoting: ISO 32000-2's "341 of 1023 pages
carry `NoZoom`" is what the *files* state, and most of those 341 are the 211 strike-outs
§12.5.6.10 clears. `zoom_cost` says so now, and the superset is still the right driver.

## Ledger

§12.5.3 and §12.5.6.4 carry what this round did and what it found; §12.5.6.10 gains
`annotation.rs` beside `appearance.rs`, because the precedence choice ADR 0172 made has one home
now. §12.5.6 was the second track's row — a `partial` whose note named nothing owed, which is
`doc/todo/01`'s first sweep shape — and it says now that what keeps it `partial` is its children
rather than its own text, with the count of them.

## Gates

`doc/todo/02` §2 whole, green: formatting and clippy under `RUSTFLAGS="-D warnings"` over the
workspace and over `fuzz/`, `nextest` (2852 run, 2852 passed, 18 skipped), the workspace doctests,
the corpus gate, the oracle, the three text-extraction gates, both censuses, dates, xmp, jpeg2000,
the quorra corpus (957 pages, 932 agree, 22 differ, 3 refused, 17 not comparable), fixed documents,
and `cargo test -p conformance` (218). §5's binaries rebuilt and installed. `pointers` and
`quotations` re-run; their hits are the standing ones.

`doc/todo/00`'s step 7 is not owed: nothing this round did changes what is drawn, and
`tests/replacement.rs` is the assertion that it does not.
