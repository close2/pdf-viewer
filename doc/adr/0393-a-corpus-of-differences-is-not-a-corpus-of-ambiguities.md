# 0393 — A corpus of differences is not a corpus of ambiguities

Status: accepted
Date: 2026-08-17
Session: 558

Takes `doc/todo/03` §13's named successor — `pdf-association/pdf-differences`' 37 documents, the
last unranked population on this disk — and first answers the decision §4 has held it behind since
the four-hundred-and-twenty-second session: what verdict a page gets when conforming readers
legitimately differ. Amends §9.6.4's ledger row.

## Context

`doc/todo/03` §4 posed the question and predicted the answer:

> `pdf-differences` exists *because* readers diverge on its files, so it is the population where a
> reference comparison should be most informative and where the three references are least likely
> to form a consensus. That is a session of its own and it needs a decision first: an oracle run
> over files chosen for disagreement will produce `ambiguous` almost everywhere, so the verdict
> vocabulary may not fit.

The worry behind it is sharper than the sentence: a page where this tree differs from three
references **by the standard's own permission** is not a contradiction, and recording it as one
would teach the ratchet a falsehood.

## Decision

### 1. The premise is wrong about this corpus, and reading it is the first thing the round owes

The name says *differences*, and it means **implementations** differ, not that the standard permits
them to. Sixteen of the eighteen test cases quote a normative sentence of ISO 32000-2 and then say
which rendering is correct; the repository's own README states the convention outright — "Correct
renderings are always the _last_ image in the MarkDown". The PDF Association built this corpus to
*end* differences, and every directory ships the answer beside the question.

Exactly two things in it are differences the standard sanctions, and both say so in its own words:

- §8.4.3.4's added paragraph — "A zero length dash occurring at a zero length subpath segment does
  not have a determinable direction and thus, if the line caps are non-round is rendered in an
  implementation-dependent manner" — which `Dashing-Degenerate`'s README paraphrases as "there is
  no right or wrong rendering for the situation described";
- §9.5 NOTE 5's font substitution, which reaches `OverlappingGlyphClipping` and `VerticalText`
  because neither embeds a font, and which the first of the two READMEs admits in advance.

A third is adjacent and is not about pixels: `UnknownFilter`'s README says "[t]he core PDF
specification (ISO 32000) does not prescribe user experience behavior", which is a silence about
*messages* rather than about marks.

**So the population is 18 clause readings with 37 hand-built witnesses, and the standard answers
16 of them.** Any decision taken on the assumption that it is a bag of ambiguities would have been
taken about a corpus that does not exist.

### 2. `pdfref::Outcome` does not gain a verdict

Three arguments, and the first is the one that would still hold if the corpus were what §4 assumed.

**A verdict must be a function of what the instrument measures.** All four existing outcomes are:
`NotEnoughReferences` counts rasters, `Ambiguous` asks whether two of them agree, `Agrees` and
`Regression` ask where we sit relative to a consensus that exists. "The standard permits this
difference" is a function of a *clause*, and no raster contains one. A vocabulary whose fifth term
its own instrument cannot compute is a term assigned by hand — and, being the only comfortable one,
it becomes where every page nobody wants to explain goes. That is the escape hatch `CLAUDE.md`
forbids the ledger one directory over, arriving in the oracle instead.

**`ambiguous` already means the right thing, and it does not mean what the worry assumes.** Its
definition is a statement about the *evidence*, not about the standard: the references disagree, so
there is no consensus to hold us to. A page where the standard permits difference and the
references duly differ is precisely a page where their agreement would have carried no information
— which is what `ambiguous` says. It is not a weaker `contradicted`; it is "this instrument has
nothing to say", and that is true.

**The permission belongs where the clause is, and the tree already has that place.** `oracle.rs`'s
named groups are a verdict plus a clause plus a measurement — `CONTRADICTED_SUBSTITUTED_FONT`
argues from §9.5 NOTE 5 and has since it was written. A permitted difference is held by a group
whose name and doc comment cite the permitting clause. That is not a new verdict; it is the
existing mechanism used for what it was built for, and it keeps the asymmetry principle 5 wants:
**a permission has to be quoted to be claimed.**

**The sharp case, stated so it cannot be softened later: a page where we differ from three agreeing
references by the standard's own permission stays `contradicted`.** That verdict says "two or more
references agree and we differ", which is a true statement about the evidence and the one fact a
later round needs. Downgrading it would hide that three independent implementations chose
otherwise. What converts an accusation into a documented choice is the group's quotation, not the
verdict's name.

### 3. What the corpus is for: a documented set of readings, and not an oracle population

**It may not go through the oracle's vote.** On this corpus the references are the *subject under
test* — the files were selected because implementations split on them — so voting them is reading
the answer off the very programs the corpus was assembled to catch out. Principle 5 forbids that in
general; here it is not even weak evidence, because the population is *selected* for the vote being
split. This round's own numbers show what that is worth: on `IndexedColor`,
`Inline-Image-Abbreviations`, `ColorBurn`, `ColorDodge` and `SelfIntersecting-Transparency` one
reference is wrong against the standard, and on `Type3WordSpacing` **two of the three are** — the
majority reading is the wrong one, and a vote would have ratified it.

**It can become a gate per case, and only per case.** Each README states an invariant that is
derivable from the clause it quotes and needs no reference at all: *both rows shall match exactly*
(§8.6.6.3), *the same image in all eight locations* (§8.9.7), *if you see a green rectangle that is
a FAIL*, *the tip of the mitre is `w/(2·sin(φ/2))` above the join* (§8.4.3.5). That is the same
shape as `format-corpus`' one-line ink assertion, and it is the transferable half: a diagnostic
corpus is gateable exactly where the clause supplies the expected value.

**Today it is a reading list, and that is its highest-value use**: 18 clauses, each with a
hand-built witness and a published statement of the intended appearance. That is the spec-driven
track `CLAUDE.md`'s two tracks asks for, arriving with its own fixtures.

## The chunk

`tools/safedocs survey --dir doc/corpora/pdf-differences`: 37 documents, 0 unopenable, 0 locked, 0
encrypted beyond us, 0 pageless, 0 slow. The reported count is one above
`doc/oracle-and-corpus.md` §2's row and both moves are new reports rather than regressions —
ADR 0356 took `UnknownFilter-ICC.pdf` to complete on Table 65's own recovery, and ADR 0359 made a
damaged form `XObject` loud, which names `UnknownFilter-FormXObject.pdf`. **Run the survey with the
sandbox worker on disk**: without it two JPEG 2000 images are refused and the line reads two higher,
which is the security posture working and not a property of the files.

Then page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the
page box (trap 3), ranked by our ink minus the lightest live reference's, with session 554's audit
column beside it because the ink ranking is blind to a page drawn on the wrong sheet.

**The ranking's head is real and is two orders of magnitude clear of the rest**, which is the shape
session 505 saw and session 554 did not: `OverlappingGlyphClipping.pdf` at **−8.989**, then
`CutHereExample.pdf` −1.237 and nothing else past −0.6. The positive tail is +2.399
(`Negative-DashPhase.pdf`, our zero-width grid lines drawn as §8.4.3.2's one solid device pixel
where `mutool` and `gs` grey them), +1.570 and +1.193 (a reference lighter than the other three).

**And the audit column found one row again**: `LineCap-Degenerate.pdf` is 4000 × 4000 here, in
`mutool` and in `gs`, and 400 × 400 in `poppler` — the page states Table 31's `/UserUnit 10`, which
this tree has read since the twenty-ninth session and `poppler` does not.

## Every document that differs, with its clause

| case | files | clause | which of the three |
|---|---|---|---|
| Atomic-Fill+Stroke | 3 | §11.7.4.4 | ours matches the published correct image; `poppler` composites the overlap |
| ColorBurn-ColorDodge | 2 | §11.3.5.2, Table 134 | ours matches both published images; `poppler` uses the ISO 32000-1 formulae |
| Dashing-Degenerate | 1 | §8.4.3.4 | the standard's own permission; the ranking separates nothing (−0.042) |
| Dashing-EndBeforeBend | 2 | §8.4.3.6 | agrees |
| DefaultColorSpaces | 2 | §8.6.5.6, §7.8.3 | agrees, both the stated and the inherited variant |
| IndexedColor | 1 | §8.6.6.3 | agrees — both rows match exactly; `poppler` blacks the out-of-range patches |
| Inline-Image-Abbreviations | 1 | §8.9.7 | agrees — all eight identical; `poppler` fails six of the eight |
| **LargeMitreLimit** | **2** | **§8.4.3.5** | **our defect** — `doc/todo/11` §6 |
| LineCap-Degenerate | 1 | §8.5.3.2 | agrees; `mutool` draws nothing. Table 31's `/UserUnit` is `poppler`'s gap |
| Negative-DashPhase | 2 | §8.4.3.6 | agrees; `CutHereExample`'s −1.237 is §9.5 NOTE 5 (`FoxitDingbats` against `D050000L`) |
| NegativeFontSize | 1 | Table 103's NOTE | agrees |
| **OverlappingGlyphClipping** | **1** | **§9.3.6 NOTE 2, §9.5 NOTE 5, §9.6.2.2** | **permitted, and our choice inside the permission is bad** — `doc/todo/21` §6 |
| PageLabels-UX | 1 | §12.4.2 | agrees — all sixteen labels, including the four the file spells with a prefix and no `/S` |
| PDF-version | 3 | §7.5.2 | agrees — all three resolve to 1.6, which is the corpus's stated answer |
| TextClipModeChanges | 1 | §9.3.6 | agrees |
| **Type3WordSpacing** | **1** | **§9.6.4, Table 111, §9.3.3** | **our defect — fixed in this round** |
| UnknownFilter | 11 | §7.4, §7.5.7, §7.5.8 | ten agree or report; **`UnknownFilter-Linearized.pdf` is our defect** — `doc/todo/17` |
| VerticalText | 1 | §9.7.4.1 | agrees |

Two of the eleven `UnknownFilter` files are unmodified real-world documents whose README withholds
the diagnosis as a challenge. It is an image declaring `DCTDecode` and carrying PNG bytes — this
tree reports `Illegal start bytes:8950`, which is `\x89P`, and draws nothing; `mutool` sniffs and
draws it. The README calls displaying it **incorrect**, so we are on the side the corpus names.

## The defect this round fixed: §9.6.4's two colours

`Type3Test.pdf` sets a blue non-stroking and a red stroking colour and shows two `d1` glyph
descriptions that stroke — one a dashed square (`S`), one a dashed filled triangle (`b*`). The
published correct appearance strokes them red. This tree stroked them blue, because `d1` collapsed
the two colour parameters into one:

```rust
state.stroke_colour = state.fill;
state.stroke_pattern = state.fill_pattern.clone();
state.stroke_alpha = state.fill_alpha;
```

The argument written beside it was Table 111's singular "[i]ts colour shall be determined by the
graphics state in effect each time this glyph is painted by a text-showing operator", supported by
the clause's reason for admitting an image mask. **§9.6.4 refutes it three times.**

- The sentence that anticipates exactly this description lists what it must set for itself — "if it
  invokes any operator from "Table 59 -Path-painting operators" which performs stroking, it shall
  explicitly set the line width, line join, line cap, and dash pattern to appropriate values" — and
  puts **no colour in that list**, because the inherited one is what it gets and §8.6.8 forbids it
  from stating another.
- NOTE 2 is **plural**: "Normally, it is unnecessary and undesirable to initialise the current
  colour parameters because the text-showing operators are designed to paint glyphs with the
  current colours."
- §9.6.4's own **EXAMPLE** states `0.2 0.8 0.0 rg 0.1 0.4 0.0 RG` before each of its three `Tj`s,
  for a `d1` `square` glyph whose whole description is `72 w 0 0 750 750 re B` — a fill *and* a
  stroke — and Figure 62 draws the border in the second colour. Under the collapsed reading those
  three `RG`s are dead syntax in the standard's own example.

The image-mask sentence the old comment leaned on is §8.9.6.2's rule about a *stencil*, which
paints with the non-stroking colour because that is what an image mask does — not because a `d1`
glyph has one colour.

`d1` now does only what §8.6.8 says it does: the description's own colour operators are ignored.
Both inherited parameters stand, and each is used by whichever operation selects it.

**`poppler` and `ghostscript` collapse the two exactly as this tree did; `mutool` does not.** The
test that pinned the old reading said so in its own doc comment — "`poppler` and `ghostscript` read
it this way, `mupdf` uses the stroking colour" — and chose the majority. That is the curve-fitting
principle 5 forbids, wearing a clause argument, and it is the best evidence this round produced for
§3 above: on this corpus a vote is not evidence.

## What moved

One document of the 974 changes its display list, and it is the pdf.js corpus's own copy of the
same file: `Type3WordSpacing.pdf`, same command count, same list length, different paint. Pixels
move, so the quorra lanes and `doc/todo/00` step 7's ink sweep were run rather than argued away.

## What this leaves

- `doc/todo/11` §6, `doc/todo/17` and `doc/todo/21` §6 carry the three findings this round did not
  take, each with its witness and its closed form.
- **`doc/todo/03` §1's rule needs no successor named for the first time in nine chunks**: every
  population on this disk is now ranked. What is left is `doc/todo/03` §1's other standing offer,
  SafeDocs' 31 GB issue-tracker corpus — or the *gates* this corpus makes possible, which are
  cheaper and are named in §3 above.
