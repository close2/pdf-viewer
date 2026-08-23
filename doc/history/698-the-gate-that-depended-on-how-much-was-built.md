# 698 — The gate that depended on how much was built

Fourth merge round of the block. Four branches, one conflict, and one finding of the merge round's
own that is larger than anything the four rounds carried: **a gate's verdict depends on how much of
the workspace was built**, and this tree has been running that gate in the scope where it passes.

## The sequence, whole, on a quiet machine (load 1.18)

`fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 ·
`nextest` **2505 passed, 18 skipped** · conformance 182 + 5 + 1 · `cargo deny` all four ok · corpus
**974 documents, 67 incomplete** · oracle **1945 pages — 983 agrees, 65 contradicted, 832 ambiguous,
42 not comparable** · `render-quorra` **957 pages — 932 agree, 23 differ** · `fixed_documents` 40/0 ·
text, selection, dates, XMP, JPEG 2000. Ledger 875 rows, **443 implemented, 224 partial, 0
unreviewed** — 696's §8.5.2.1 moving `implemented` → `partial`. Negatives queue **34 done, 14 owed**
over a population of **48**, which is two larger than last round's 46 because two rows gained a
negative sentence. §5's binaries rebuilt and installed.

**The oracle failure three rounds inherited is gone**, and 694 is why.

## The merge round's own finding

695 reported that `accessibility_census` fails on `main` and not on its branch, and that a scratch
worktree at its branch point *passed* until it was rebuilt in a `target-dir` of its own, whereupon it
failed. It recorded that as trap 16 with the mechanism unestablished, and **it did not lower the
floor** — which was the right call twice over, because the mechanism is worse than a stale directory.

Measured here, four ways:

| | reads | verdict |
|---|---|---|
| shared build directory | **1336** | passes |
| clean directory, subset built | **1345** | **fails** |
| clean directory, after `cargo clean -p` on four crates | 1336 | passes |
| clean directory, **whole workspace built** | **1336** | passes |

Deterministic in each, twice over, and the two test binaries have different digests. So it is **not**
the directory and **not** staleness: it is **build scope**. Cargo unifies features across whatever is
in the build, and building the whole workspace gives a dependency a feature set that building
`viewer-core`'s subset does not — and the program then *counts nine structure elements differently*.

**This makes session 660's report real, and this project's record of it wrong.** 660 said
`cargo nextest run -p pdf-model` alone fails six CCITT tests where `--workspace` passes, and blamed
feature unification resolving `hayro-ccitt` differently under a scoped build. Session 664 checked it
on merged `main`, found 1099 passing, and recorded it as **not reproducing**. That check was run in a
fully-built shared directory, which is precisely the scope where the defect hides. *A claim that a
defect does not reproduce is a claim about the conditions you reproduced it under*, and 664's
conditions were the ones that conceal it.

Three consequences, none of them settled here:

- **The gate is not measuring the program a user gets.** `cargo build --release --bin pdf-viewer` is
  a *third* scope, and nothing has established which of the feature sets the shipped binary carries.
- **Which number is right is unknown.** 1345 and 1336 are two different readings of 974 documents;
  the ratchet's floor was set under one of them and nobody has asked which.
- **A feature that changes what the program computes should not be reachable by unification.** If a
  dependency's behaviour turns on a feature, this workspace should state it, not inherit it from
  whichever crates happen to be in the build.

That is a round's work and it is named as one. `doc/HANDOVER.md` and trap 16 carry it, with 695's
observation kept and its diagnosis corrected.

## 694 — the reference that was not there

The batch's other instrument finding, and it arrived as a *baseline* failure on a tree the round had
not edited. 688 had removed `function_based_shading_cmyk.pdf page 2` from its contradicted group on
the conclusion that "the consensus dissolved". It had not:

- the figure the removal rests on, **29.06**, is `poppler` against `mupdf` to the hundredth;
- `mupdf` against `ghostscript` is **0.192%** — a consensus by any bound the gate holds;
- all three cached panels are byte-unchanged since 2026-07-29, and today's `pdftoppm`, `mutool` and
  `gs` reproduce them to four decimals with the gate's own arguments;
- our own printed numbers are session 680's table unchanged.

**The run that reported no consensus had no `ghostscript` reading in it**, and `render_references`
discarded a failed reference *silently* whenever two others remained — so the line it printed was
indistinguishable from a page judged on three. The absence is now printed on the page's own line and
counted in the summary; the run finds **six such pages, all previously invisible**. Trap 9 gains a
ninth mechanism, and it is the sharpest of the nine: not two references agreeing for a bad reason,
but *one reference not being there* and the report not saying so.

Its subject went the other way. The **63 `pdfbox` pages are all diagnosed and the file is empty
again**, in two groups, and the strongest result is a replication: §3c's finding that the bound sits
below the spread of the implementations that set it reproduces on a corpus **with no part in setting
it** — the smallest reference-to-reference differing fraction exceeds the 5% bound on all 63, and our
worst mean is at or below the largest reference-to-reference mean on 61 of 63. It also corrected
692's filing of that shape (692 said mean and worst tile were comfortable; counted off the gate it is
mean 47 of 63 and worst tile 4) and found that the printed line is `poppler`'s on 53 and
`ghostscript`'s on 10 and never `mupdf`'s — because the best whole-pixel offset to `mupdf` is (0,0)
on all 63 and to `poppler` it is one device row on 50.

## 695 — the sentence the standard does not contain

`doc/todo/30` item 3, `viewer-ui`'s password prompt, and **both native hosts carried in quotation
marks a sentence ISO 32000-2 does not have**: *"the interactive PDF processor **shall** … prompt the
user for a password"*. §7.6.4.1 says **`should`**. The quotation gate could not see it because it
reads rustdoc blockquotes and these were `//` comments — **a misquote that lived by virtue of comment
syntax**. And the misreading was load-bearing: NOTE 2 defines the processor that genuinely cannot ask
as a *non-interactive* reader, "such as printing off-line or on a server", and a window is not one,
which is what makes the old `exit(1)` a misreading rather than a rough edge.

**The password was one struct field's declaration order from a launch log.** `Command` derives
`Debug`, both native hosts trace with `format!("{command:?}")` truncated to 120 characters, and
`bytes` happens to precede `password`. `viewer_core::Secret` now redacts in `Debug`, has no
`Display`, zeroes on drop, and reserves §7.6.4.1's own 127-byte truncation point so an edited
password never reallocates into memory it cannot clear. **`zeroize` was refused** — a volatile write
is `unsafe` and both crates `forbid` it — and the weaker guarantee is documented rather than bought
by weakening the ban. That is principle 1 and principle 3 deciding against each other in the open.

Three defects only a screen could report, all in the no-page frame path that had not existed before:
a missing `adopt` costing **1196 frames in 20 seconds**, an unconditional clock arm, and an overlay
list that could not clear the card. Plus two toolkit traps — `GtkWindow::close` fires
`close-request` **synchronously**, so every GTK password was being reported as a decline.

## 696 and 697 — the paperwork that was a defect, and the note that contradicted itself

**696 found a real drawing defect through the negatives queue.** §8.5.2.1's negative is false: a
segment operator with no current point occurs on 12 documents and **5010 operators** over the curated
corpora's first hundred pages, and `tiny_skia::PathBuilder` injects a move to the origin — so **the
page gets a line nothing asked for**. Row `implemented` → `partial`. The `h` half of the same
sentence costs nothing, because `close_subpath` declines an empty path: one clause sentence, two
costs. Its §9.4.2 reading is the batch's neatest — **false in the half it states and true in the half
it means**: a document does move `Tm` between a `q` and its `Q`, but Table 106 makes the next `Tm`
replace what the `Q` restored, so restores *a mark can see* are zero on every well-formed page.

Its new `operator_shape_census` answers claims about an **order** of operators, which neither
`witness_census` (a name or a token) nor `absence_audit` (a structure) can reach — and it prints the
**37 685 streams it does not walk**, which is the honest way to publish a census's blind spot.

**697 found a note that contradicts itself.** The paragraph that *narrowed* §11.6.6's row in session
492 named a document's "inner gray-`ICCBased` groups" as its witness; all eight are the `/G` of an
`/SMask` `/Luminosity` dictionary — §11.5.3's population, which **ADR 0276 had removed from this
clause four paragraphs earlier in the same note**. Its §11.4 family was chosen for *shape*: those
rows count each other, and all four cross-references were wrong. Two rules out of it, and the second
is new:

> **A corrected row is not a safe row** — the relapse was written by the round that narrowed it. And
> **no instrument can see this**: all eighteen sweeps compare a row with the tree, a row with its
> children, or a row with the standard, and this contradiction is between two paragraphs of one note.

## Errata: the seventh, eighth and ninth, and two more reasons the checker is blind

- **#134** (697) inserts "of the transparency group XObject" into Table 145's `/CS` row, settling
  which resource dictionary remaps a group's device colour space. **#74's shape a third time**: our
  code was right for its whole life *on no stated authority*.
- **#373** (696) corrects Table 106's `T*` row from `TD` to `Td` — **a one-word strike, below
  `check`'s four-word floor**, which is a *second* blindness independent of the Caret-with-no-
  `StrikeOut` shape that accounts for the others.
- **`emit` files an annotation by the page its outline puts in a clause**, and §11.6.6's heading sits
  at the *bottom* of its page — so ADR 0492 filed two neighbouring carets under the wrong clause and
  never saw #134 beside them. The filing shape and the rect arithmetic are now written down.

## Owed

- **The feature-unification round**, above: which scope the shipped binary has, which of 1345 and
  1336 is right, and whether a behaviour-changing feature should be stated rather than unified.
- **`issue19083.pdf`** on the cross-backend gate (23 differ) — our CPU render moved toward the
  references and quorra's did not. `doc/QUORRA_FEEDBACK.md` §24b.
- **Twelve of the fourteen negatives left are noise or not corpus claims** — 696 measured that, so
  the queue will finish smaller than its count.
- **`viewer-ui` still `exit(1)`s** on `Event::OpenFailed` and on a zero-page document — the same
  shape 695 fixed for passwords, named rather than changed.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
