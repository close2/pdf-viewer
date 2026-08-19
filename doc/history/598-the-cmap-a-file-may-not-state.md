# 598 — The CMap a file may not state

Both tracks, and they turned out to be the same clause.

## Demand-driven: the head of `doc/todo/00` step 7

Ran the oracle gate whole, then step 7's ink sweep myself over all 786 ambiguous pages the gate
printed — our ink minus the lightest reference's, references with zero ink dropped first, the
corpus's incomplete labels carried into the loop.

**19 at or past −1, 16 of them documents this tree calls incomplete.** Head:

```
-19.447  issue12418_reduced.pdf   ours 0.000 vs hayro       19.447  [incomplete]
-13.810  issue4722.pdf            ours 0.000 vs mupdf       13.810  [incomplete]
-12.927  issue15977_reduced.pdf   ours 0.000 vs poppler     12.927  [incomplete]
-11.272  bug1050040.pdf           ours 0.000 vs hayro       11.272  [incomplete]
 -8.991  issue5801.pdf            ours 0.000 vs ghostscript  8.991  [incomplete]
```

On the complete documents `issue16038.pdf` −5.737, `issue12295.pdf` −2.363, `issue14297.pdf`
−1.130, then `issue7821.pdf` −0.957 and nothing past −0.840 — three past −1 and all three
diagnosed, which is the alarm holding again.

**The head is one cause and it had never been opened.** `examples/open_one` on each of the top
names prints the same report, and it prints it on eleven of the sixteen incomplete names in the
negative tail: a Type 0 font with `/Encoding /Identity-H` over a `CIDFontType2` whose descriptor
carries no `/FontFile2`, with no `/ToUnicode` and `/CIDSystemInfo` `(Adobe) (Identity) 0`. The
side-by-side strips say what that costs: ours blank, and the four references drawing four
*different* strings from the same twelve codes.

§9.7.5.2 forbids the combination outright — "The Identity-H and Identity-V CMaps shall not be
used with a non-embedded font" — and §9.7.4.2 says why a reader has nothing to work with. So the
refusal is the clause and not a gap, and the finding is that nothing in the tree had said so.
ADR 0433 has the reading, the eleven names, what each reference guesses, and the numbers.

**The price the parent asked about is zero, and it is read off the code.**
`tools/pdfref/src/lib.rs::decide` returns `Outcome::Ambiguous` when no mutually-agreeing subset of
two or more references exists — before our comparison is consulted at all. Measured pairwise, the
closest *voting* pair on those eleven pages is 4.63 to 16.51 of 255 apart against a text
tolerance of 5.00, so every one would be `ambiguous` whatever this tree drew. What a report does
cost is a place in `oracle.rs::check_the_ratchets`, whose `named` filters on `e.complete`; the
corpus gate's `Text` row holds the same population from the other side.

## Spec-driven: §9.7.5.2

`spec-errata emit` over `doc/*.pdf` first, per `doc/todo/02` §4: two annotations under §9.7,
neither touching the sentence above. The clause's ledger row was `implemented` and quoted the
processor's obligation about character collections; it now also quotes the `shall not` about the
*file*, names the eleven pages it settles and points at the ADR. `loading.rs`'s refusal carries
the same sentence beside it.

## What did not change

No rendering code. Adopting the standard-Macintosh-glyph-ordering reading two of the references
use would be curve-fitting against a clause that says CIDs shall not participate in glyph
selection, which principle 5 forbids outright. `ours 0.000` on those pages is the answer.

Because nothing that draws moved, the sweep needs no after-half: the artefacts the gate wrote are
the ones it measured.
