# 467 — The operands that immediately precede

**Finding.** §7.8.2 says "all of the operands needed by an operator shall immediately precede that
operator", and this interpreter read them from the *front* of everything the stream had stated since
the previous operator rather than the back. On a conforming stream the two are the same slice, which
is why four hundred and sixty-six rounds and every gate in the tree saw nothing; on a malformed one
they differ completely. The witness draws a blank page **in silence**:
`T02-05-01_008_Font-set-operator-missing.pdf` deletes a `Tf` keyword and leaves `/F0 36.` standing in
front of a `Tj`, so `string_at(operands, 0)` found a name, the show operator disappeared before
`show_text` could report it, and no report existed to be lost. Read from the back it is the string,
the show runs with no font, and §9.3.1's lost mark is announced.

**Date.** 2026-08-13.
**ADR.** [0302](../adr/0302-the-operands-that-immediately-precede.md).
**Touched.** `crates/pdf-model/src/content.rs` (`operands_before`, `count_of`, the accumulator
renamed `pending`, and forty-four call sites losing a borrow),
`crates/pdf-model/tests/leftover_operands.rs` (new),
`doc/conformance/ledger.toml` (§7.8.2), `doc/oracle-and-corpus.md` (§2's table, §2b, §2c),
`doc/todo/03-more-corpora.md`, `doc/todo/README.md`,
`doc/adr/0302-*`, this file.

## The chunk, and why this one

`doc/todo/03`'s standing item is *take a chunk a round*, and SafeDocs' `CC-MAIN-2021-31` has been
surveyed whole since the four-hundred-and-thirty-third — there is nowhere in it nobody has been. So
the chunk was a different corpus: `openpreserve/format-corpus`, sparse-checked out to its five PDF
directories, 267 files, 702 MB, fetched into the `.gitignore`d `corpus-cache/` and never staged.
`tools/safedocs survey --dir` over each of the five is the whole instrument and needed no new code.

**`pdf-handbuilt-test-corpus` is the reason the chunk paid.** Its 89 files each carry *one*
deliberate structural defect and all draw the same *Hello PDF-world!*, so the survey's five questions
are not the measurement — the **ink** is. Rendering every file at scale 1 and taking the mean, the
intact page reads **0.807367** and sixty-odd files reproduce it exactly; **fourteen read 0**, of
which nine say why. **Five are blank in silence**, two of them rightly — Table 31 makes a page
stating no `/Contents` empty, and a file whose show operator was deleted has nothing to show — and
the three that remain are the finding: a page tree node with no `/Kids`, a `Tf` whose size operand is
a lone `.`, and a deleted `Tf` keyword, which is the one this round took. A corpus of 0.1 MB found
what 93 GB of crawl could not, because it was built to be diagnostic rather than large — which is now
written into `doc/todo/03` §1 as the lesson.

## What it moves

`examples/display_list_digest` on both sides: **2** of the 974 change, **0** of 4000 crawled web
documents, 0 of the 267, 0 of `doc/corpora/`'s 108. That is §7.8.2 predicting its own reach — a
producer that leaves an operand over has written an invalid stream — and it is why the change is
safe to make everywhere at once.

Both of the two were opened (trap 1). `issue6342.pdf` is pdf.js's *Form XObject with errors* and its
form stream is corrupt from byte 1300, so its `c` operators run with junk in front of them: ours
painted a fat green blob and now paints a thin crescent, **which is the shape `mupdf` paints**, while
`poppler` stops at the first bad keyword and paints nothing there. Principle 5's direction only — the
clause decided it and mupdf agreeing raises confidence the clause was read right.
`poppler-90-0-fuzzed.pdf` gains six commands.

## The gates

The whole of `doc/todo/02` §2 ran after the last edit. The corpus gate's incomplete list is
**65**, unchanged; the oracle is 905 agree / 68 contradicted / 786 ambiguous, unchanged, at a 99.7%
cache hit rate; both text gates unchanged (99.3% of `pdftotext`'s words over the 974, 99.8% against
PDFBox's frozen extraction); quorra 919 agree / 37 differ / 1 refused, unchanged; dates, xmp and
jpeg2000 unchanged; conformance clean over 6843 citations with the new ledger quotation verified
against `doc/md/`. `cargo nextest` is 1669 tests, seven of them new.

The forty-four call sites are the change's only risk and clippy is what found them: renaming the
accumulator made `&operands` a double reference, so `needless_borrow` named every site that had to be
visited. A silent change would have been a worse one.

## What the round did not do

- **Decide the licence.** `openpreserve/format-corpus`'s five directories were *read* — only
  `pdfCabinetOfHorrors` states CC0 in its own sidecar, `govdocs1-error-pdfs` states other terms
  outright, and `jhove-errors` is ninety-nine published journal articles under no sidecar at all. The
  question for the owner is one sentence in `doc/todo/03` §2 and the evidence is
  `doc/oracle-and-corpus.md` §2c. No submodule was added and nothing in the tree depends on the
  corpus.
- **Report the leftovers.** §7.8.2 makes such a stream malformed, so a report is available; trap 11
  is why there is not one. After the fix the mark is drawn correctly, so the report would name no
  lost mark while taking every page carrying one off the oracle's judged set.
- **Take the two other findings.** A `/Pages` node with no `/Kids` yielding a silent blank page, and
  a `Tf` whose size operand is a lone `.` setting a size of zero — both in `doc/todo/03` §7 with what
  each wants before it can be taken, which in both cases is a population nothing counts yet.
