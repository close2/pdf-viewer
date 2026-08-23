# 0541 — A vote is evidence only where a clause is being read

Status: accepted
Date: 2026-08-23
Session: 692

Puts the oracle in front of the four corpora `doc/corpora/` pins, decides which two of them its
vote is evidence about, and gates those. Amends `doc/oracle-and-corpus.md` §2 and §2d, and
`doc/todo/03` §4. Adds `AMBIGUOUS_HIGHLIGHT_APPEARANCE_STREAM`; extends
`CONTRADICTED_SUBSTITUTED_FONT` and `CONTRADICTED_GLYPH_EDGES`.

## Context

`CLAUDE.md`'s two questions give robustness one denominator — *the world* — and one instrument,
the corpus and the oracle. For six hundred sessions that instrument's population has been one
project's bug reports: `doc/pdf.js/test/pdfs`, 974 documents, plus page one of the specification
PDFs. Four more corpora arrived as submodules between the four-hundred-and-twenty-second session
and the four-hundred-and-seventieth (ADRs 0258, 0305) and **every one of them has been surveyed
and none has been voted on.**

The distinction matters and the briefing this round was given had it collapsed. `doc/todo/03`
§§8, 12, 13 and 14 record all four as *ranked*, and they were — by a per-round script that renders
page one at 72 dpi against `pdftoppm`, `mutool` and `gs` and sorts by our ink minus the lightest
live reference's. A ranking finds the head of a distribution. It reaches no verdict, holds no
page by name, and leaves nothing behind that fails a build; `doc/todo/03` §20 says as much when it
asks a chunk to leave "a file rather than a memory". What none of the four had ever been given is
`pdf-model`'s oracle gate: `Judgement::CORPUS`'s bound derived from the references' own spread on
that page, `pdfref::Outcome`'s four verdicts, and the ratchets that hold every non-agreeing page
by name in both directions.

So the second question had been answered over 974 documents and asked over 275 more.

## Decision

### 1. The vote goes to `pdf20examples` and `pdfbox`, and the reason is not their size

`pdfref`'s rule is ADR 0005's: two implementations sharing no code agreeing about a page is
evidence about the specification. **The inference has a precondition that is easy to lose — there
has to be a clause they are both reading.** Where there is not, their agreement is a fact about
three programs and about nothing else, and principle 5 forbids treating it as more. That
precondition, and not cost, is what splits these four populations two and two.

**`doc/corpora/pdf20examples` — voted.** Seven files the PDF Association publishes to demonstrate
ISO 32000-2, written by the people who wrote the clauses, valid by construction, each built to
exercise a named feature. Three independent readers of a clause agreeing about a file built to
exercise *that clause* is the strongest form the triangulation rule takes anywhere in this tree.
Eight pages.

**`doc/corpora/pdfbox` — voted.** Apache PDFBox's own regression inputs: every file is there
because it broke a PDF library once, which is exactly why the 974 are there, and like them they
are overwhelmingly *valid* documents a reader got wrong rather than malformed ones the standard
says nothing about. This tree has read the corpus since ADR 0259 for `PDFTextStripper`'s frozen
text and had never pointed a raster at it under a verdict. 143 pages, all pages rather than page
one — and that choice paid inside one run: `unencrypted.pdf` is contradicted on page **2** and
agrees on page 1.

**`doc/corpora/format-corpus` — not voted, and this is the entry that is a clause reading rather
than a preference.** All three of its pinned directories are deliberately damaged files: 89
hand-built documents carrying one structural defect apiece, 24 archival horrors, 54 crawled `.gov`
documents that broke somebody else's software. `CLAUDE.md` states the governing fact outright —
"[r]eal files are malformed, truncated and written by twenty-year-old generators; the standard
describes *valid* files and says nothing about the rest". On a file whose cross-reference table is
wrong, whose catalogue is missing or whose page tree is recursive, **there is no clause for three
programs to agree about.** §7.5.8.4 and §C.4 permit a reconstruction and state no algorithm for
one, so agreement there is agreement between three recovery heuristics — trap 9's shared gap with
the gap made deliberate by the corpus's author.

That is not an argument for looking away, and the census below is the reason it is not. It is an
argument that this population's verdicts may not be *ratcheted*, because a ratchet turns a
contradiction into a thing to be made to go away, and `CLAUDE.md` is explicit that "we differ from
poppler on a corrupt file" is a question. The population also has a better instrument already, and
it needs no reference at all: every one of the 89 hand-built files draws the same *Hello
PDF-world!*, so the corpus states its own expected value (`doc/oracle-and-corpus.md` §2b).

**`doc/corpora/pdf-differences` — not voted, and ADR 0393 decided it.** The files exist *because*
implementations split on them, so the references are the subject under test and a vote reads the
answer off the very programs the corpus was assembled to catch out. Nothing here reopens that; what
this round adds is that the decision is now *checkable* rather than merely recorded — see §3.

### 2. What a voted population costs, and the one price worth naming

The two voted corpora add 151 pages to 1794. Every non-agreeing page among them is now held by
name in exactly the way the 974's are, and 64 of them are `ambiguous` — which means
`crates/pdf-model/tests/ambiguous_undiagnosed.txt`, **empty since the three-hundred-and-seventy-ninth
session, is no longer empty.**

That file's own comment already answers whether that is a regression: "Empty is a state this file
can be in and not a state it stays in." The sharper statement is the one this round owes, because
the temptation runs the other way: **a bucket kept empty by choosing the denominator is
`CLAUDE.md`'s corpus-going-quiet failure with better bookkeeping.** The emptiness was a true fact
about one population and was never a fact about this reader. Declining to measure a second
population in order to preserve the appearance is precisely the move principle 5's escape-hatch
paragraph exists to forbid one directory over, and it is worse than the ledger version because
nothing would have printed.

One of the 64 left the file immediately with a diagnosis (§4), and the shape of the rest is
visible from the census rather than guessed: 62 of the 63 are text at document sizes failing the
differing fraction and the structural similarity while sitting well inside the mean and the worst
tile, which is `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`'s and `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`'s
signature. Saying so is a hypothesis; those groups are measurements. `doc/todo/00` is where turning
the first into the second is written down, and it now has a ranked population to do it over.

**The gate's own accusing ranking is what says the price is bookkeeping rather than defects.**
`doc/todo/00`'s instrument ranks undiagnosed ambiguous pages by how far we sit from the **nearest**
reference, because that is the number that accuses us; on this run its head is **1.11** and its
tenth place 0.80. Every one of the 63 is inside about one and a tenth of the bound at its closest
reference, against a nearest of 28.91 at the head of the contradicted ranking printed two lines
below it in the same output. That comparison is only available because the file was allowed to
fill.

### 3. What is not voted is censused, and the census is a command rather than a paragraph

`oracle.rs::what_the_references_say_about_every_submodule_corpus` renders **all four** corpora,
gated and ungated alike, prints the verdict census per population and names every page that is not
an agreement. It asserts only that it had a population to print.

Three things follow, and the third is the one that makes the exclusions honest:

- It declines unless `PDFVIEWER_ORACLE_CENSUS` is set, for the reason ADR 0282 gives at length:
  `-- --ignored` is a switch on the binary rather than a filter, so the gate's own command line
  would otherwise run this beside it and pay for 219 pages on every round. The guard is in the
  test because an invocation can be copied without its guard and a test cannot be run without
  itself.
- It is `CLAUDE.md`'s "a fact that can be counted is not written down" applied to the two
  populations that have no gate: their numbers live in a command, and `doc/oracle-and-corpus.md`
  carries the argument and not the figures.
- **A `voted: false` that nothing can print is a decision nobody can check.** The census is what
  makes §1's two exclusions readable as claims: run it, and it prints what the vote would have
  said. On `pdf-differences` this round that was two contradicted pages, and both are the two
  §9.5 NOTE 5 substitutions ADR 0393 named — the decision reproducing itself, which is the only
  form of confirmation an exclusion can have.

### 4. Three pages diagnosed, and one of them is what a valid-file corpus is for

The run produced five contradicted pages and one geometry disagreement across the two voted
corpora. None is a new defect and each was taken to a clause or to an existing group's own
instrument.

**`pdfbox/PDFBOX-2984-rotations.pdf` pages 1 to 4 → `CONTRADICTED_SUBSTITUTED_FONT`.** Six pages of
one line of 50 pt `/Helvetica`, `/WinAnsiEncoding`, nothing embedded, drawn at `/Rotate` 90, 180
and 270 once through a text matrix and once through a `cm`. This is the first page that group has
taken from outside the pdf.js corpus, and **the constant the group derived predicted it**: the
capital `A` at 8× is 358 rows in ours against 379 in `poppler`'s and `mupdf`'s, and 0.6875 /
0.729167 — Liberation Sans's cap height over `NimbusSans`', measured from the two font files in
the five-hundred-and-fourteenth session — predicts 357.3. The advances are *not* what differs: the
ink's bounding box is 420 × 86 at (100, 64) in ours and 420 × 87 at (101, 63) in `poppler`'s, the
same width to the pixel over a 420-column line, which is worth stating because the page's ink
centroid moves 5.5 device pixels and reads like a shifted line. Pages 5 and 6 carry the same 8.5%
deficit and the gate calls them `agrees`, because their consensus pair sits further apart and the
bound derived from it is wider — trap 12 read from the other end, and the reason this group's
membership is a measurement and never a verdict.

**`pdfbox/unencrypted.pdf` page 2 → `CONTRADICTED_GLYPH_EDGES`.** Both its fonts are *embedded*, so
the group next door cannot reach it. The two ladders answer it: ours 6.0086 at the page's own scale
and **6.2690 at 8×**, against `poppler` 6.2305 / 6.2613 and `mupdf` 6.0988 / 6.2542. At eight times
the resolution the three agree to 0.015 of 255 and ours is inside their span. The page carries a
second mechanism — its heatmap is hollow letters *and* one-pixel edges around filled rectangles —
and that is named beside the diagnosis rather than folded into it, because trap 9's *a page can
carry two of the eight* is what a group's name is most often wrong about.

**`pdf20examples/PDF 2.0 UTF-8 string and annotation.pdf` page 1 →
`AMBIGUOUS_HIGHLIGHT_APPEARANCE_STREAM`, and this one earned the population its place.** A blank
sheet with one `/Highlight` annotation, an `/AP` drawing a yellow rectangle, and a
`/QuadPoints` array. Ink at the page's own scale: ours 0.820296, `mupdf` 0.820296, `hayro`
0.820296, `poppler` 4.81698, `ghostscript` 0. Three renderings identical to four significant
figures, two nowhere near, and no two voting references agreeing — so the verdict is `ambiguous`,
which is the instrument correctly saying it has nothing to hold us to. What the page is worth is
that **each of the other two has a different reason and the file states one of them itself**:

> The QuadPoints array here conforms to 32000-2 and therefore acts strange in readers that do not
> conform to the standard.

`poppler` synthesises the mark from `/QuadPoints` instead of drawing the `/AP`, in Acrobat's
historical vertex order rather than Table 182's "counterclockwise order", and the polygon crosses
itself. `ghostscript` draws nothing because the annotation states no `/F`, so Table 167's Print
flag is clear and `gs` renders for a printer — trap 3. We draw the appearance stream, which is
§6.3.2.2's second-named obligation on a rendering processor.

**On the standard's own seven demonstration files, the first page that is not unanimous is one
where two of three references are answering something other than the clause and we are not.** That
is the argument for §1's first entry arriving as a measurement in the same run that made it.

### 5. `corpus.rs` is deliberately not extended, and that is an argument rather than an omission

The corpus gate asks whether we *reported* everything we could not draw. `tools/safedocs survey
--dir <path>` asks the same five questions over any directory and ratchets none of them, and
`doc/oracle-and-corpus.md` §2's table has a row for each of these four from the day it was pinned.
So the self-report question was already asked of these populations and answered. What was missing
was the vote, which is what this round built. A second ratchet over the same five counts, with a
constant per population, would add bookkeeping and no question.

## What moved

**No pixel moves.** The whole change is `crates/pdf-model/tests/oracle.rs` and
`ambiguous_undiagnosed.txt` — a gate's population and its diagnoses. `Work` gains the corpus it
came from; pages out of the pdf.js corpus and out of `doc/`'s specification PDFs are named exactly
as before, which is why not one of the existing ratchet entries moved. The label is not decoration:
three of the 275 documents under `doc/corpora/` share a *file name* with one of the 974 —
`attachment.pdf`, `rotation.pdf`, `IndexedCS_negative_and_high.pdf` — and only two of those three
share their bytes, so an unlabelled name would have put two different documents in one ratchet
entry. `Work::artefact_directory` carries the label for the same reason one directory down.

The gate before this round, and after it, on the same tree and the same warm reference cache:

| | pages | agrees | contradicted | ambiguous | our geometry | reference geometry | not comparable | no render |
|---|---|---|---|---|---|---|---|---|
| before | 1794 | 902 | 60 | 768 | 2 | 2 | 42 | 18 |
| after | 1945 | 983 | 65 | 832 | 3 | 2 | 42 | 18 |

The 151 new pages are 81 agreements, 5 contradictions, 64 ambiguities and one geometry
disagreement, and they sum. The geometry one is
`pdfbox/PDFBOX-6018-099267-p9-OrphanPopups.pdf` page 1 at 596 × 842 where all three references say
612 × 792, which is ADR 0389's `Page::DEFAULT_MEDIA_BOX` on a page stating no `/MediaBox` anywhere
in its ancestry: the standard states no recovery, we report the page, and a reported page is not
gated.

## What this leaves

- **63 ambiguous pages with no diagnosis**, all `pdfbox`, ranked by the census and named in
  `ambiguous_undiagnosed.txt`. `doc/todo/00`'s method applies to them unchanged.
- **`format-corpus`'s census is not nothing**, and a later round may find it worth reading rather
  than gating: 14 of its 167 first pages are `not comparable` because the *references* refuse the
  file, which is a measurement of how much of that population is beyond any triangulation, and 5
  are pages we report and do not draw.
- **The other direction of §1's precondition is untested.** This ADR argues a vote is evidence only
  where a clause is being read; it does not follow that every voted page's agreement *is* evidence,
  and trap 9 lists eight ways it is not. The census over the two excluded corpora is the cheap
  half; the expensive half is `doc/todo/00`.
