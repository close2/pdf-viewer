# 665 — The paragraph a correction said it had not touched

The instrument 662 asked for, built and calibrated. Parallel round, worktree `r665`, branch
`round-665`. **No pixel moves**; what changed is a new sweep, three page-list notes, §10.7.4's
ledger row, trap 1 and two catalogue entries. ADR 0491 has the argument and the four candidates
measured before choosing.

## What was built

`cargo run --release -p conformance --bin overtaken` — the nineteenth sweep, the fourteenth to be
a program. Its population is the tree's **page-list notes**: the doc comment above a
`const NAME: [&str; N]` of corpus pages, which is where `oracle.rs` keeps every non-agreeing page's
diagnosis. 123 of them over 310 documents, in the oracle, quorra's corpus, `pdf-model`'s corpus and
`text_extraction`.

Its discriminator is the only date this tree keeps in machine-readable form: **an ADR number is a
date**, so a note's citations are a claim about which decisions it has read. It compares the newest
ADR a note cites with the newest ADR that names one of the note's own pages. Three rungs — the
later ADR names the list, names a page the prose argues, names only a list member — and **all three
require a shared page**, which the sweep's own first run taught it: without that, a *census* ADR
that prints every list's name put 24 of 123 notes on rung 1 and ADR 0489 was in all of them.

## Why not the other three

Measured rather than reasoned about, and two of the four premises turned out false:

- `--bin pointers` **already reads `oracle.rs`** and reports nothing there. The notes' paths and
  symbols are live; what decayed was a measurement, and a measurement names no file.
- The gate's own words appear **12 times (`ssim`) and 34 (`worst tile`) in 7523 lines of note**, so
  "a note usually quotes figures the oracle recomputes" does not hold and there is nothing to
  anchor a bare number to.
- **ADR 0476 records no supersession of 0474**, so 662's case has no supersession record to find;
  31 of 489 ADRs mention supersession at all.

A fifth was prototyped and abandoned for the right reason: *the same measurement stated twice with
two values* finds the live defect with no gate run, and is **silent on the plant**, because
restoring the stale sentence makes the tree agree with itself.

## Calibration, before any correction

`git checkout fbe65e72^ -- crates/pdf-model/tests/oracle.rs` restored 662's own defect. The sweep
names `CONTRADICTED_TIGHT_CONSENSUS` as the **first line of its first rung**, overtaken by ADR 0476
and ADR 0489 on `colors.pdf`. `--bin retired` over the noun a reader would type returns 254
mentions of `quarter` with the same sentence near rank 100.

## What its first run found

48 of 123 overtaken. Three acted on, all one page-family:

- **`CONTRADICTED_ANTIALIASED_EDGES`** (rung 2, rank 1) still gave ours as ssim 0.98591 / 0.97906
  against an exact form at 0.98772 / 0.98001 — the quarter-quantised raster's numbers, nineteen
  sessions after ADR 0476 made ours the exact form. The gate prints **0.9879 / 0.9802** against
  bounds of 0.9886 / 0.9840. It sat **directly below the ADR 0476 correction, which ends "the
  paragraph below is unaffected — which it predicted"**. A correction that scopes itself is a
  claim, and that one was false about the only sentence in its scope.
- **`CONTRADICTED_UNEXPLAINED`** (rung 2, rank 2) was still asking for a measurement 662 made, and
  gave `issue7891_bc1.pdf` as mean 0.22, one tile at 10.76, 0.52% differing. The gate prints
  **0.17, 6.73, 0.54%**. Corrected off the run, pointed at the group the page lives in, and given
  §10.7.4's clipping paragraph verbatim.
- **`CONTRADICTED_TIGHT_CONSENSUS`** (rung 1 on the current tree) is not stale — 662 rewrote it and
  did not cite its own ADR. One citation added, and the cure written into `doc/todo/02` §4 as a
  rule: **a round that rewrites a note cites its own ADR in it.**

## Changed

- `tools/conformance/src/overtaken.rs`, `src/bin/overtaken.rs` — new; `src/lib.rs` registers it.
- `crates/pdf-model/tests/oracle.rs` — three notes, doc comments only.
- `doc/conformance/ledger.toml` §10.7.4 — the second home of the claim the row had corrected once.
- Trap 1 in `doc/traps/pixels-and-rasterisers.md` — the third way a note is wrong now has an
  instrument, and the by-hand tell is kept because the sweep only ranks.
- `doc/todo/01`'s *sweeps as commands* and `doc/todo/02` §4.

## Owed

- The 45 notes the sweep still names, headed by `AMBIGUOUS_ICC_MATRIX_PROFILE` (cites ADR 0025;
  ADRs 0251 and 0488 name its pages). Each needs the later ADR read against the note.
- The 62 notes that cite no ADR at all, which the sweep counts and cannot rank.
- `doc/todo/11` item 4's group blit still needs a shape channel beside a group's raster — 662's,
  unchanged.
