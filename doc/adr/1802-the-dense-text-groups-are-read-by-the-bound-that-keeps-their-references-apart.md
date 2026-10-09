# 1802 — The dense-text groups are read by the bound that keeps their references apart

Session 1483. Status: **accepted** and **built**. Code: `crates/pdf-model/tests/oracle.rs`
(`FormationStop`, `RaisedFormation::stop`, `what_keeps_the_dense_text_apart`, and the note of
`AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`), `tools/state.sh` (`section_oracle`'s filter).

## What was asked, and what the walk said

The round was sent to take the largest disagreement group the oracle ranks that no constant holds.
On HEAD there is none. The walk printed 1026 agree, 47 contradicted, 836 ambiguous, 46 not
comparable, 8 no render, 2 reference geometry and 3 our geometry, each equal to the held count, and
`ambiguous, undiagnosed` printed no page. Six batches of drawing changes moved no page into or
out of a verdict. So the round took the brief's other branch and read the bound of the largest
ambiguous group, `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` (369 pages: `freeculture.pdf` 317 and
`pdkids.pdf` 52).

## The bound a note named is not the one that decides the verdict

The group's note reads the text tolerance's 0.90 similarity against **our** band. But a page is
ambiguous here because no two voting references form a consensus, so what decides it is reference
against reference. Measured on the gate's own panels, reconciled to one size, with
`examples/compare_rasters`, pair by pair over all 369 pages:

- all 1107 voting pairs are past `TEXT_HEAVY`'s 5.00% differing fraction, the nearest at 5.03%.
  That one bound would keep every page ambiguous on its own;
- with that bound taken out of formation, a pair forms on 313 of `freeculture`'s 317 pages and on
  9 of `pdkids`'s 52. On the other 47 pages the pairs fail the similarity or the mean as well.

So the book is ambiguous because of ADR 0243's bound, the one of the four set below the spread it
was measured on. `pdkids` is ambiguous because of a scatter that no bound of the class forgives.
§10.7.4's "[s]can conversion of character glyphs may be performed by a different algorithm from
the preceding one" covers both and tells them apart for neither.

## The decision

1. **The gate prints it.** `FormationStop` takes, on every page with no consensus, the voting pairs
   past the class's differing fraction. It also takes whether `Triangulation::rejudged`, run with
   that bound taken out of formation, forms a set. That keeps `decide` as the only rule for
   whether a set forms. `what_keeps_the_dense_text_apart` prints it for the three dense-text groups,
   which together hold 609 of the 836 ambiguous pages, and `tools/state.sh oracle` keeps the line. It is a
   census and not a ratchet. Every number in it is reference against reference, so nothing in this
   tree can move it. A reference release or a change to the population can.
2. **The note cites that line** and keeps the two-document split from the calibration above. The
   band of our own measures was re-derived from this run's per-page lines. It had moved by up to
   0.29 of 255 on the mean and 0.41 points on the differing fraction since ADR 0685.
3. **The bound does not move.** If a consensus formed without the fraction, what it would conclude
   is ADR 0776's counterfactual. On this run that counterfactual convicts 272 of these pages, nearly
   all of them on the similarity and mostly through a pair that is not the page's closest reading.
   A later round that finds the book "held back by one bound" has found this ADR, not a lever.

## Calibration

The calibration is trap 13's: the printed line has to reproduce the scratch census, which was
taken by a second instrument (artefact PNGs and `compare_rasters`, not the gate's in-memory
comparisons). Both give 322 pages where a set forms, and both give every pair as past the
fraction. Re-judging at a page's own bounds reproduces its verdict, as `a_raised_formation_bound`
already asserts.
