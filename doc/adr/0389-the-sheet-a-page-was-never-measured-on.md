# ADR 0389 — The sheet a page was never measured on

Status: accepted, 2026-08-16. Session 554. Takes `doc/todo/03` §12's named successor — `pdfbox`'s
64 documents, the larger of the two populations that file leaves unranked — and makes loud a
substitution this tree had been making in silence since its first page tree. Amends §7.7.3.3's and
§7.7.3.4's ledger rows.

## The chunk, and why this one

`doc/todo/03` §12 closes by naming what is left: "[w]hat is unranked on this disk after this chunk
is `pdfbox`'s 64 and `pdf-differences`' 37; the second of those is §4's first bullet and still
wants its decision about the verdict vocabulary before anybody runs it." So the file names its own
next chunk and there was no choice to make: **`pdfbox`'s 64**, Apache-2.0, a submodule already
pinned, and the one of the two that needs no decision first.

It is the right *kind* of chunk by §1's own finding as well. `pdfbox/src/test/resources/input` is
the regression corpus of another PDF library: every file in it is there because it broke something,
which is the diagnostic property §1 says outranks size. And this tree already reads it for one
thing — §2a's frozen `PDFTextStripper` comparison over the 40 that carry a `.pdf.txt` — while
nobody had ever pointed a *raster* at it.

## The survey, and the ranking

`tools/safedocs survey --dir`, the tree's own instrument, reproduces `doc/oracle-and-corpus.md`
§2's row for this corpus exactly: 64 documents, 0 unopenable, 0 locked, 0 encrypted beyond us, 0
pageless, **1 incomplete**, 0 slow — and the one is `MAX_FORM_DEPTH` on
`PDFBOX-4372-…-p4_reduced.pdf`, which that table has recorded since the corpus arrived. A clean
population through the survey's eyes.

Then page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the
page box (trap 3), ranked by **our ink minus the lightest live reference's** — `doc/todo/00` step
7's number on a population with no ambiguous bucket, the way sessions 505 and 544 applied it.

**The negative tail separates nothing**: −0.410 (`survey.pdf`), −0.328 (`tiger-as-form-xobject`),
−0.205 (`PDFBOX-2984-rotations`) and shallower, and the three deepest were opened side by side —
one page four times, differing in glyph weight. The positive tail is the same story with the
reference at fault rather than us: `PDFBOX-3127-…-VFont` at +1.332 is `gs` drawing the same
notice-of-public-hearing 20% lighter than the other three.

**What the ranking found is in a column beside the number rather than in the number.** The script
prints each panel's raster size, because trap 3's tell is a dimension and not a difference — and
one row of the 64 has ours at 596 × 842 where all three references say 612 × 792:
`merge/PDFBOX-6018-099267-p9-OrphanPopups.pdf`. Its gap is *none*, because both pages are blank:
the file's two `Text` annotations set Table 167's Hidden bit, so every renderer correctly draws
nothing. A page that everybody draws blank at two different sizes is invisible to a ranking by ink,
and the instrument caught it anyway because the sizes are printed. **A ranking's own audit column
found what the ranking could not.**

## The reading

The page object is four entries long — `<< /Annots 22 0 R /Type /Page /Parent 161 0 R >>` — and its
`/Pages` node is `<< /Count 1 /Kids [21 0 R] /Type /Pages >>`. Neither states `/MediaBox`, and
neither states any other rectangle.

§7.7.3.3 Table 31 makes the entry

> ( Required; inheritable ) A rectangle (see 7.9.5, "Rectangles"), expressed in default user space
> units, that shall define the boundaries of the physical medium on which the page shall be
> displayed or printed (see 14.11.2, "Page boundaries").

and §7.7.3.4 says where an inheritable required entry may be written instead of on the page:

> If such an attribute is omitted from a page object, its value shall be inherited from an
> ancestor node in the page tree. If the attribute is a required one, a value shall be supplied in
> an ancestor node.

The file breaks both sentences, and **the standard states no recovery for it**. Every one of
§14.11.2's other four boundaries is defined *in terms of* the media box — Table 31 defaults the
crop box to it and §14.11.2.1 intersects the other three with it — so there is no route from the
rest of the page dictionary back to a size, and nothing else in clause 7 or clause 14 states a
default. This is the honest limit `CLAUDE.md` names: where the standard defines nothing, done means
a documented choice.

**The choice was already made and already documented.** `Page::DEFAULT_MEDIA_BOX` has been A4 since
the page tree was written, with the reason beside it. What had not been decided is what to *say*.

## Decision

**Keep A4. Report the substitution.**

Two halves, and the first is the one worth arguing.

**A4 stays, and that is principle 5 rather than inertia.** All three references draw such a page
612 × 792, and on `T02-03_009_page-object-mediabox-not-rectangle.pdf` — a `/MediaBox [0 612 792]`,
three numbers where §7.9.5 requires four — they return *three different answers*: `pdftoppm`
612 × 792, `mutool` 792 × 612, `gs` nothing at all. Moving a constant to match a vote on a question
the standard answers nowhere is curve-fitting with extra steps, and the vote is not even unanimous
under perturbation. What the disagreement earns is a report, not a new constant.

**The substitution is reported**, and it is the eleventh place this program reports while drawing.
Trap 5's test — suppressing either statement loses information — is met in both directions:

- Refusing the page throws away marks nothing else in the file can supply. The crawl's one witness
  is a 22-page arithmetic worksheet that draws perfectly well.
- Saying nothing makes a page whose size is a guess of ours indistinguishable from a page its
  producer measured. On `1407606.pdf` the guess puts the producer's content **50 points lower** on
  the sheet and cuts **16 units** off its width, and no metric in this tree could see it, because
  ink is a fraction of the page and both numerator and denominator moved together.

**It is not ADR 0106's additive-or-substitutive question**, and the difference is worth stating
because that test decides most of trap 5's other entries. What is substituted here is not a *mark*
but the frame every mark is measured in. A missing line end decorates a line the clause draws
anyway; a guessed media box relocates the entire content stream.

**The two ways to reach it are counted apart**, because they are different mistakes by the producer
and a reader that conflated them would send a person looking for an entry that is there:
`MediaBoxSubstitution::Absent` — no node in the ancestry writes `/MediaBox` — and
`::NotARectangle` — one writes it and no node writes §7.9.5's four finite numbers.

## The population, measured before the decision and not after

`crates/pdf-model/examples/media_box_census.rs` walks the page tree with `pdf_syntax` alone,
because a census whose predicate is the code under test measures the code (trap 8). One process per
archive over the crawl, the surveys' own method. It is a baseline for this population, never a
ratchet, and the command prints today's:

| population | documents | pages | page one |
|---|---|---|---|
| the 974 (964 open) | 4 | 4 | 4 |
| `doc/corpora/pdfbox` | 1 | 1 | 1 |
| `doc/corpora/format-corpus` | 4 | 4 | 3 |
| `pdf20examples` + `pdf-differences` | 0 | 0 | 0 |
| **the crawl, 65 703 that open, 919 979 pages** | **1** | **22** | **1** |

Three things it says.

- **The crawl's rate is 1 in 65 703**, which is 0.0015% of the web — the rarest population
  `doc/todo/03` has measured. It is also a *whole document*: `1407606.pdf` is 22 pages, every one
  of them with `/Contents`, every one of them displaced.
- **Two of `format-corpus`' four are built to carry exactly this defect and nothing else** —
  `T02-03_008_page-object-mediabox-missing.pdf` and `T02-03_009_page-object-mediabox-not-rectangle.pdf`,
  one per branch of the enum above. `doc/todo/03` §7 recorded the handbuilt corpus as *spent*
  because all five of its silent blanks were accounted for; this is the correction that entry
  earns, and it is a general one: **the instrument was spent for the question it was pointed at.**
  Those two files never drew blank. They drew *the wrong page*, which is a defect no blank-page
  assertion can see, and it took a different question — asked from a different corpus — to reach
  them.
- **Not one page of the whole population states another of §14.11.2's boxes.** That is the check
  that says the substitution discards nothing: had a producer written a `/CropBox` and no
  `/MediaBox`, it would have said how big its page was and a guess would be throwing the file's own
  words away. None did, anywhere on this disk.

## What it costs

**Nothing the oracle was judging.** Of the four in the 974, `poppler-937-0-fuzzed.pdf` is already
unusable, `issue9105_other.pdf` and `operator_list_cycle.pdf` are already incomplete for other
reasons, and `issue15590.pdf` is the one document the report newly makes incomplete — and all three
references refuse it, so it has no reference render and was never in the judged set. Trap 11's
question — cost a report in gated pages — answers **zero** here, which is as cheap as a new report
gets.

The corpus gate's incomplete count therefore rises by one, and trap 5's rule applies without
qualification: a rise that is a new report is not a regression.

**No pixel moves.** Nothing about the substitution changed — the same A4 rectangle, the same
display list — which the display-list digest over all 974 first pages confirms byte for byte. So no
quorra lane and no ink sweep were run, on `doc/todo/00` step 7's own condition.

## What this leaves

`doc/todo/03` §12's second name, `pdf-differences`' 37, is still unranked and still wants §4's
decision about the verdict vocabulary first. And `doc/todo/21`'s question is untouched: this says
the *sheet* is a guess and says nothing about the page-one raster of a document whose sheet is
stated.
