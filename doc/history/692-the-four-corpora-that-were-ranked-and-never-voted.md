# 692 — The four corpora that were ranked, and never voted

The round was asked to put the oracle over `doc/corpora/`'s four submodules and to decide what
should be gated. The first thing it found is that the question had been recorded as answered four
separate times and was not.

## What the briefing had, and what the tree had

`doc/todo/03` §§8, 12, 13 and 14 each record one of these populations being "put in front of a
reference", and §14 concludes "[e]very population on this disk is ranked". All four sentences are
true and none of them says what the round was asked for. **Ranked means the ink ranking** — page
one at 72 dpi against `pdftoppm`, `mutool` and `gs`, sorted by our ink minus the lightest live
reference's, run by a script that lives for one round. It finds the head of a distribution. It
reaches no verdict, holds no page by name, and leaves nothing that can fail a build; §20 of the
same file says a chunk should leave "a file rather than a memory", and these four chunks left
memories.

**Voted means `pdf-model`'s oracle gate**, which none of the 275 documents had ever been through:
`Judgement::CORPUS`'s bound derived from the references' own spread on that page, `pdfref::Outcome`'s
four verdicts, and ratchets held to equality in both directions. So `CLAUDE.md`'s second question
had been answered over 974 documents and asked over 275 more.

The briefing was also stale in the other direction: it said "what is left is the oracle over the new
corpora", and `doc/oracle-and-corpus.md` §2d records that decision taken for one of the four
(ADR 0393, `pdf-differences`) with an argument this round did not reopen. Both halves are now
written down where a later round will meet them.

## The population

275 documents, of which **272 are new to this tree's judged population**: three share their bytes
with a document of the 974, and one more — `pdfbox`'s `attachment.pdf` — shares only its *name*
with one, which is why every page from a submodule corpus now carries its corpus's label.

`tools/state.sh` prints the counts; ADR 0541 has the argument. What is gated is
`pdf20examples` and `pdfbox`; what is censused is all four.

## The decision, in one sentence

**A vote is evidence only where there is a clause the references are both reading**, which is
ADR 0005's precondition rather than a new principle, and it splits the four two and two.
`pdf20examples` is the standard's own demonstration files and `pdfbox` is another library's
regression inputs — valid documents a reader got wrong, which is what the 974 are. `format-corpus`
is deliberately damaged files, and `CLAUDE.md` says the standard "describes *valid* files and says
nothing about the rest": three programs agreeing about a file whose cross-reference table is wrong
agree about three recovery heuristics. `pdf-differences` stays out on ADR 0393's own argument.

**Both exclusions are printed rather than asserted.**
`what_the_references_say_about_every_submodule_corpus` renders all four and names every page that
is not an agreement, declining unless `PDFVIEWER_ORACLE_CENSUS` is set (ADR 0282's guard, in the
test rather than in the invocation). A `voted: false` nothing can print is a decision nobody can
check — and on the run that built it, the pages it called contradicted in `pdf-differences` were
exactly the two §9.5 NOTE 5 substitutions ADR 0393 had already named.

## The three pages it diagnosed

None is a new defect and each went to a clause or to an existing group's own instrument. The
measurements are in the group comments in `oracle.rs`; the shapes are:

- **`pdfbox/PDFBOX-2984-rotations.pdf` pages 1–4** — `CONTRADICTED_SUBSTITUTED_FONT`, and the
  group's own constant *predicted* it. Liberation Sans's cap height over `NimbusSans`' is
  0.6875 / 0.729167 = 0.942857, derived from the two font files in the five-hundred-and-fourteenth
  session; the capital `A` at 8× on this page is 358 rows against the references' 379, and
  379 × 0.942857 = 357.3. The advances are not what differs — the ink box is 420 columns wide in
  both — which needed saying, because the page's centroid moves 5.5 device pixels and reads like a
  shifted line. Pages 5 and 6 carry the same deficit and *agree*, because their consensus pair sits
  further apart: trap 12 read from the other end.
- **`pdfbox/unencrypted.pdf` page 2** — `CONTRADICTED_GLYPH_EDGES`, both fonts embedded, settled by
  the two ladders: at 8× the three renderers agree to 0.015 of 255 and ours is inside their span.
  The page carries a second mechanism and it is named beside the diagnosis rather than folded in.
- **`pdf20examples/PDF 2.0 UTF-8 string and annotation.pdf` page 1** —
  `AMBIGUOUS_HIGHLIGHT_APPEARANCE_STREAM`, and it is what a valid-file corpus is for. Ours,
  `mupdf`'s and `hayro`'s ink are identical to four significant figures; `poppler` draws a bow-tie
  and `ghostscript` draws nothing, for two different documented reasons. `poppler` synthesises from
  `/QuadPoints` in Acrobat's vertex order instead of drawing the `/AP`, which the file's own
  comments predict — "The QuadPoints array here conforms to 32000-2 and therefore acts strange in
  readers that do not conform to the standard" — and `ghostscript` is right about the question it is
  being asked, the annotation stating no `/F` and `gs` rendering for a printer (trap 3). We draw the
  appearance stream, which is §6.3.2.2's second-named obligation.

## The one price, and why it was paid rather than avoided

`crates/pdf-model/tests/ambiguous_undiagnosed.txt` had been **empty since the
three-hundred-and-seventy-ninth session**, down from 754 names. It is not empty now: 63 `pdfbox`
pages are in it.

Not one is a regression. Every one is a page nobody had ever judged, and the emptiness was a true
fact about *one population* that was never a fact about this reader. The alternative — declining to
measure a second population so the file stays empty — is `CLAUDE.md`'s corpus-going-quiet failure
with better bookkeeping, and it is worse than the ledger version because nothing would have
printed. 62 of the 63 are text at document sizes failing the differing fraction and the structural
similarity while well inside the mean and the worst tile, which is
`AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`'s signature; saying so is a hypothesis and those groups are
measurements, so `doc/todo/00` now has a ranked population and not a claim.

**And the ranking the gate prints is the reassuring shape rather than the alarming one.** The
oracle's own "ambiguous, undiagnosed, and furthest from the **nearest** reference" table — the one
`doc/todo/00` calls the accusing measurement, because a page far from even its closest reference is
a page where we differ from everybody — has a head of **1.11** and a tenth place of 0.80. Every one
of the 63 sits inside about one and a tenth of the bound at its nearest reference. Compare the
ranking that *is* alarming, which the same run prints two lines down: on the contradicted list the
nearest is 28.91. A queue whose head is 1.11 is a queue of measurements owed and not of defects
hidden, and that is a statement the file could not have made before it was allowed to fill.

## What did not move

**No pixel.** The change is `crates/pdf-model/tests/oracle.rs` and one data file — a gate's
population and its diagnoses — so no quorra lane and no ink sweep were owed. Pages from the 974 and
from `doc/`'s specification PDFs are named exactly as before, which is why not one existing ratchet
entry moved.

`corpus.rs` is deliberately not extended, and ADR 0541 §5 is the argument: the self-report question
is already asked of these populations by `tools/safedocs survey --dir`, and what was missing was the
vote.

## The machine

Heavily loaded throughout and shared with other rounds — load average between 5 and 50 over the
session, on 24 cores. Every wall-clock figure the round printed is therefore a measurement of the
machine as much as of the tree, and the verdict counts are not: the reference cache was the shared
warm one at `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`, whose entries are keyed on the
invocation and the document's digest and written through a per-process temporary name, so sharing
it between worktrees is safe and changes no verdict.
