# 0934 — What was left of the matrix that cancels the page

Session 942. Status: **accepted**. ISO 32000-2 §12.5.6.22's `/FixedPrint`, applied nine sessions
after ADR 0906 found that it was a display obligation rather than a printing one.

## Context

`/FixedPrint` is the entry that makes a watermark a watermark: Table 194's `/Matrix`, `/H` and
`/V` state where the mark goes *on the media*, and §12.5.6.22's second bullet substitutes the
result for the rectangle §12.5.5's algorithm places an appearance onto. Two reasons have been
given here for not applying it, and both were about this program rather than about the standard:

| session | the reason on record | what was wrong with it |
|---|---|---|
| before 933 | "a resolution-independent display list cannot express a size that depends on the view" | ADR 0168 had already dissolved that for §12.5.3's two flags |
| 933 | "it waits on a printing path rather than on a display one" | the clause introduces the entry with a `shall` on *rendering* and states the on-screen media dimensions itself (ADR 0906) |

ADR 0906 left the entry **reported** rather than applied, and named the residue precisely: two of
the transformation's three terms are stated outright, and the third — "a matrix B that maps a
scaled and rotated page into the default user space", to be cancelled — "is stated against a media
origin whose relationship to this tree's page space is a derivation nobody here has made". This is
that derivation, and the finding is that it is smaller than the sentence.

## Decision

**Compute §12.5.6.22's transformed annotation rectangle and hand it to §12.5.5's algorithm in
place of `/Rect`.** `annotation::fixed_print` is the whole of it, and `annotation::decided` passes
its result to `placement`. The substitution is an *argument* to an algorithm this tree already
runs for every annotation, which is what the clause's own wording asks for — "it shall be used in
place of the annotation rectangle referred to in steps 2 and 3" — rather than a second placement
mechanism beside §12.5.3's.

### The three terms, and which of them the standard states

1. **The rectangle through `/Matrix`.** Stated outright. Which corner goes to the origin is stated
   too, one clause over: step 2 of the algorithm this substitutes into is about "the lower-left
   corner (the corner with the smallest x and y coordinates)", and `annotation::rectangle` has
   already normalised `/Rect` onto it. The smallest upright rectangle around the result is step
   1's own construction, so `transformed` computes both.
2. **`/H` and `/V`.** Stated outright, as percentages of the media's dimensions, and §12.5.6.22
   says which media a screen has: "When displaying a watermark annotation on-screen, interactive
   PDF processors shall use the dimensions of the media box". Table 193 says the same for a
   processor that does not know its target. (The on-screen sentence cites "Table 29 -Entries in
   the catalog dictionary", which is the catalogue and states no media box at all; Table 31's page
   entry is what the other two references name.)
3. **The cancellation of B**, and here is the finding. B is what places a page onto a *sheet* — the
   paragraphs after the EXAMPLE say so, opening "[i]n situations other than the usual case where
   the PDF page size equals the media size" and going on to page tiling and n-up printing. The
   on-screen sentence makes the page's media box **be** the media, so a screen is that usual case
   by construction: B's scale and rotation are the identity *by the clause's own stipulation*,
   not by a choice. What is left of the sentence is the translation between two origins, and
   §8.3.2.3's NOTE 1 is why that is not nothing: "the origin of default user space always
   corresponds to the lower-left corner of the output medium … it is not required". So the term is
   the media box's lower-left corner — nought for every file whose media box starts there, and the
   whole difference for one whose does not.

**A percentage of a width is a distance, which is what makes term 3 belong to term 2.** `/H` and
`/V` have to be measured from *somewhere*, and the only corner in the clause is the media's. The
sentence about B is the standard saying where that corner is relative to default user space; on a
screen it says nothing else.

### The one choice

**§7.7.3.3's `/Rotate` is not cancelled.** §12.5.6.22 never names the entry; what it says of a
screen is that the behaviour is "the same as for other annotations"; and §12.5.3's `NoRotate` is
the flag whose whole purpose is to depart from turning with the page. Reading a second such
mechanism into the B sentence would make that flag redundant on this one subtype, and would leave
a watermark unable to ask for the behaviour its own `/F` states. Recorded as a choice because the
clause does not settle it in words, and argued rather than assumed.

The same scoping settles the rest of the interaction without a second decision: the substitution
names **steps 2 and 3**, so §12.7.4.3's substitute `/BBox` — step 1's operand — and §12.5.3's fixed
point, which §12.5.5 applies "further" after step 3, both keep the rectangle the file states.

## The population, and what it can and cannot say

`crates/pdf-model/examples/fixed_print_census` was written before the code, over every PDF in this
tree rather than over the gate corpus alone. Of the **4172 that open**, five state a watermark
annotation and **exactly one states a `/FixedPrint`**: `isartor-6-5-2-t01-fail-d.pdf`, of the
Isartor PDF/A-1b suite. The 974-document gate corpus states none; neither do the four
`doc/corpora` submodules. RFC 0004's survey had said "essentially absent"; this is the number.

**That one witness checks the arithmetic and cannot rank it**, which is worth separating. Its
`/Rect [148.75 272.25 446.25 569.75]` on a 595 × 842 media box, under `/Matrix [1 0 0 1 -148.75
-148.75]` with `/H 0.5` and `/V 0.5`, transforms to **exactly its own `/Rect`** — the producer
wrote the two rectangles equal — so every one of the four terms above has to be right for the mark
not to move, and its raster is byte-identical before and after this change. A page whose picture
does not move cannot fail, which is why `the_one_witness_transforms_onto_its_own_rectangle` stays
green with `fixed_print` planted to answer `None`, and why the three fixtures that *do* move are
hand-built (trap 8). The test says that calibration out loud rather than implying a discrimination
it does not have.

## Consequences

- `crates/pdf-model/tests/annotations.rs` gains four tests, all of which rasterise and measure the
  mark rather than asserting a matrix (trap 1): the translation case, a rotating `/Matrix` — whose
  point is that the substituted rectangle is *upright*, so the mark's proportions change and its
  orientation does not — the media box whose corner is not the origin, and the witness. The first
  three fail with the substitution removed.
- `annotation::fixed_print_owed` and its report are gone, with the departure they named. ADR
  0906's test `a_watermarks_fixed_print_is_reported_and_a_plain_one_is_not` is replaced by the
  four above; §12.5.6.22's and §12.5.3's ledger rows carried its name and now carry theirs.
- `ViewGeometry` gains the page's media box beside its `/Rotate`. Both are the *file's* geometry
  that an annotation's placement needs and the annotation dictionary does not have, which is what
  that type is for; the magnification beside them is the reader's, and stays the only one.
- The three fixture builders in `tests/annotations.rs` had three copies of the same
  cross-reference table writer; they share `assemble` now.
- §12.5.6.22 stays `partial`, and the note says what for: the two bullets after the EXAMPLE are
  printing, conditioned on a selection a program with no print path never makes. RFC 0004 carries
  them, and the two halves are now one substitution apart — the paper's dimensions in place of the
  media box's.

## The rule this leaves

**A blocker that names a term the standard has not stated is a claim to check against the
clause's own scope.** ADR 0906 read this one honestly — it said which term was underived and
declined to guess it, which is trap 5 done right. What the guess would have cost was never the
issue; what the *reading* cost was nine sessions, because nobody had asked which situation the
sentence was written for. The paragraphs after an EXAMPLE are part of the clause, and here they
name the case outright: B is the matrix of a page placed on a different sheet, and a viewer whose
media are the page's own has already been told that case does not arise.
