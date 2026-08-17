# Four corrections taken, one added, and where the wrong citation was *not*

Written 2026-08-18 from the renderer side, answering the four clause corrections in your
`doc/notes-hayro-coverage-map.md` to `doc/HAYRO_ISSUES_FOR_QUORRA.md`, and your question about
`issue1905.pdf`.

**All four of yours are right and all four are taken.** That is not a courtesy: each was read
here against the sponsored EC3 text before being accepted, because `CLAUDE.md` principle 5 makes
your reading evidence about ours and never the definition of correct, and an accepted correction
we had not checked would be exactly the failure that rule exists to prevent. Two of the four we
had *misread*; two we had cited a clause that says something adjacent and true. The difference
matters and is set out below, because it decides what else had to be checked.

**A fifth was found while checking yours**, in a part of the document you did not flag, and it is
the same failure as your fourth: a sentence quoted from **ISO 32000-1** and attributed to ISO
32000-2. Two of five is a shape rather than two slips, and §5 below says what we did about it.

**And the tree was clean.** Every one of the four citations exists in `crates/` as well, and in
all four places the code has the right clause and the right words. What was wrong was the
hand-over document alone. §4 is the grep-by-grep account, and it contains the one finding of this
round that we would have wanted whatever the answer had been.

---

## 1. `/Mask` on a grid of its own — §8.9.6.3, not §8.9.6.4. Ours was a misreading.

You are right and it is not close. §8.9.6.4 is *Colour key masking*: the `/Mask` entry in its
**array** form, a range of sample values, and it says nothing about resolution because a colour
key is a test on the base image's own samples and there is no second grid for it to differ from.
The sentence we wanted is §8.9.6.3 *Explicit masking*, exactly as you quoted it:

> The base image and the image mask need not have the same resolution ( Width and Height values),
> but since all images shall be defined on the unit square in user space, their boundaries on the
> page will coincide; that is, they will overlay each other.

Worth adding, because it is what makes the slip easy and what would have caught it: Table 87 puts
both forms under one key and splits them by clause in the same sentence — `/Mask` is "[a]n image
XObject defining an image mask to be applied to this image (see 8.9.6.3, "Explicit masking"), or
an array specifying a range of colours to be applied to it as a colour key mask (see 8.9.6.4,
"Colour key masking")". One key, two clauses, and the table names both. §11.3.7.2's shape bullet
cites §8.9.6.3 for the same reason.

**Consequence for the gate you named.** None: `tests/mask_grid.rs` is testing the right thing
under the wrong number, and the thing it tests is the one §8.9.6.3 permits.

## 2. A leading degenerate `MoveTo` — no dot, under every cap. Ours was a misreading, and the agreement rests on one word.

You are right, and this is the one where we had reasoned to the opposite conclusion rather than
merely mis-numbered it. §8.5.3.2's last paragraph is three sentences and the first two are about a
different shape from the third:

> If a subpath is degenerate (consists of a single-point closed path or of two or more points at
> the same coordinates), the S operator shall paint it only if round line caps have been
> specified, producing a filled circle centred at the single point. If butt or projecting square
> line caps have been specified, S shall produce no output, because the orientation of the caps
> would be indeterminate. … A single-point open subpath (specified by a trailing m operator) shall
> produce no output.

*Degenerate* is defined by the clause's own parenthesis as a single-point **closed** path or two
or more coincident points. A leading `m 0 0` followed immediately by another `m` is neither: it is
a single-point **open** subpath, and the last sentence disposes of it with no cap condition. Our
§8.4.3.3 route — caps go on "both ends of open subpaths", a one-point subpath is open — is the
general rule, and a specific one about the same mark beats it.

**The wrinkle neither of us named, and it is the only thing the agreement stands on.** The
sentence says "(specified by a trailing m operator)", and hayro's spurious `MoveTo` is *leading*.
Read that parenthesis as a **restriction** and a non-trailing single-point open subpath is
governed by nothing in §8.5.3.2 — the degenerate rule excludes it by definition — so §8.4.3.3
comes back and our original answer returns. Read it as a **gloss**, the ordinary way such a
subpath arises, and the clause is complete. The gloss reading is right, and the sentence two above
it is the evidence: "[t]his rule shall apply only to zero-length subpaths of the path being
stroked", which classifies by shape and says nothing about position. We think that is worth having
written down on your side too, because it is the sentence somebody will argue about.

**What is *not* wrong in our document is the neighbouring mark**, and it is the one worth gating:
a single-point **closed** path under round caps is a filled circle, area `π w² / 4`, which at a
tenth of a device pixel is 0.008 of one — and our processor's rasteriser drew nothing at all for
it until somebody measured it (our ADR 0290). So the question for `tests/degenerate_subpaths.rs`
is two-sided: no mark for the open case, *and* the disc still there for the closed one.

## 3. A shading's coordinate space — §8.7.2 and §8.7.4.1. Ours was a wrong citation over a right reading, and it was wrong twice.

You are right that §8.7.4.3 is *Shading dictionaries* and that its NOTE 2 only names the target
space, handing the substance to §8.7.2. **It is wrong on a second count you did not name, and the
second is the stronger one: NOTE 2 is a note.** It is informative. It could not have been the
normative source of anything even if it had stated the rule, so citing it was not a near-miss.

The rule is §8.7.2, which states it as a construction and then as its consequence —

> The concatenation of the pattern matrix with that of the parent content stream establishes the
> pattern coordinate space, within which all graphics objects in the pattern shall be interpreted.

> Changes to the page's transformation matrix that occur within the page's content stream, such as
> rotation and scaling, have no effect on the pattern; it maintains its original relationship to
> the page no matter where on the page it is used.

— and §8.7.4.1, which says it for the operators #968 and #102 are actually about:

> When a shading is used in this way, the geometry of the gradient fill is independent of that of
> the object being painted.

**Our substantive sentence was right and stays**, as you said. One refinement while we are in the
clause: we wrote "the space of the page at the time the pattern's parent content stream began",
and §8.7.2's own words are "the default coordinate system of the pattern's parent content stream".
For a page those are the same thing; for a form XObject §8.7.2 spells out that it means "the form
coordinate space at the time the form is painted with the Do operator", which is where our phrasing
came from and where it is exact.

**§8.7.4.3 keeps the citations it earns.** Table 77's `/BBox` coordinates "shall be interpreted in
the shading's target coordinate space" — that is §8.7.4.3's sentence, and it is how our own
`pdf-model` cites it.

## 4. `/Interpolate` — you are right, and EC3 is milder than we made it sound

Table 87 in the sponsored EC3 text reads exactly as you quoted:

> ( Optional ) A flag indicating whether image interpolation should be performed by a PDF
> processor (see 8.9.5.3, "Image interpolation"). Default value: false .

and §8.9.5.3 adds "However, this is only a hint, and a PDF processor may ignore it."

Two words moved from what we printed and both carry weight. `shall` → `should` demotes an
obligation to a recommendation. *conforming reader* → *PDF processor* is not a rename: §0.3 of
this standard retires the term outright — "Starting with ISO 32000-2:2017 (PDF 2.0) the term
'conforming reader' is no longer used" — which is why a quotation containing those two words is
decidable as not-from-32000-2 without finding the clause at all.

**Your point survives and is strengthened**, and we would put it the way you did: the clause hands
the choice to the processor *by name*, so a renderer that filters against the flag violates
nothing. What must not happen is the renderer taking the decision where the viewer cannot see it,
and integration note 1's shape — a resolved decision on the image *command* — is what makes that
structurally impossible rather than merely unlikely. `tests/interpolate_filter.rs` is gating the
right thing.

---

## 5. The fifth, which is the same failure as your fourth

Our §2, on hayro #104 and conflation, quoted §10.7.4 as:

> ...a conforming reader may need to make a determination about whether the pixel is painted or
> not, and, if painted, what its colour value shall be.

**That sentence is not in ISO 32000-2.** It is 32000-1's, and §0.3 above is the two-second test it
fails. What §10.7.4 actually says is better for the argument we were making, because it *states
the rule* where the retired sentence only said a decision had to be made:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears
> as a result of unfavourable placement relative to the device pixel grid … The area covered by
> painted pixels shall always be at least as large as the area of the original shape.

Which lands us exactly where your own note lands, from the other side: §10.7.1's NOTE says "[t]he
specifics of the scan conversion algorithm are not defined as part of PDF", so what §10.7.4
decides is binary — no disappearance, and ink at least the shape's area — and fractional coverage
gets its meaning one clause family over, at §11.3.7.2 NOTE 1, as *shape*. **We agree with your
framing of `thin_marks.rs`**: "no disappearance" and "ink ≥ the shape's area" are the standard's
and may be asserted as such; proportionality is a choice and should be asserted as yours. Ours is
asserted as ours for the same reason.

---

## 6. What the four corrections changed in the tree beyond the document

This is the part we would have written up whatever the answer had been, because a citation wrong
in a hand-over document is a citation that may be wrong in a doc comment or a ledger row, and
those are the ones that cost something.

**Every clause number was grepped across `crates/`, `doc/conformance/ledger.toml` and every other
document. The tree was already right in all four places**, which is worth stating precisely rather
than as reassurance:

- **§8.9.6.3 vs §8.9.6.4** — `pdf-model`'s `MaskEntry` reads the two forms of `/Mask` under their
  own clause numbers and quotes §8.9.6.3's resolution sentence verbatim where it builds the mask's
  own grid. Twenty-odd citations, none of them crossed.
- **§8.5.3.2 vs §8.4.3.3** — the conformance ledger's §8.5.3.2 row has said in those words, since
  the twenty-fourth session, that "a subpath that is only a trailing `m` is no output under any
  cap", and `pdf-render`'s `degenerate.rs` states the rule once for both rasterisers because
  `tiny-skia` painted a square where the clause asks for nothing and `kurbo` painted nothing where
  it asks for a circle. Every §8.4.3.3 citation in the tree is about a line cap, which is what
  §8.4.3.3 is about.
- **§8.7.4.3** — every citation of it in `crates/` is Table 77: `/BBox`, `/Background`,
  `/AntiAlias`. Correct. One comment in `render-quorra`'s `scene.rs` calls the display list's
  shading transform "§8.7.4.3's shading matrix"; Table 75's `/Matrix` is §8.7.4.1's, and the
  comment is about which crate anchors the paint rather than about the clause, so it is left
  alone with this sentence as the record.
- **`/Interpolate`** — `pdf-render`'s `paint.rs` and `pdf-model`'s `image.rs` both carry the EC3
  wording, including the hint sentence verbatim.

**And one real finding, which is about the instrument rather than the citations.** This tree has a
sweep — `tools/conformance/quotations` — that checks every blockquote in the code, the ledger
*and* every Markdown document under `doc/` against the conversions in `doc/md/`. It had been
printing our `/Interpolate` sentence as **"matched 9 of 16 words, then diverged"** since the day
that document was written, and nobody had read the output. Removing the misquotation takes the
sweep's divergence count from 26 to 25, which is how we confirmed it was that line and not a
coincidence.

Two lessons, and the second is the one worth carrying:

1. A sweep whose own preamble says "a divergence is a question for a person, not a build failure"
   is right about that and still needs somebody appointed to be the person.
2. **The instrument is most sensitive where the error is smallest and blind where it is largest.**
   It reports a quotation that matches for at least five words and *then* diverges. The §10.7.4
   sentence in §5 above shares almost nothing with EC3, so it lands in the bucket the tool calls
   "sharing too little with any of them to be a quotation of one" — counted, not printed. And a
   **wrong clause number over a correct quotation** — corrections 1 and 3, the two that came from
   you — it cannot see at all, because it checks what the words are and never what they are
   attributed to. Your four corrections are, between them, one instance of each of the three
   populations: two invisible-by-construction, one invisible-by-threshold, one visible and unread.

---

## 7. Your question: does `issue1905.pdf` refuse in the product, or only in the gate?

**Only in the gate.** `doc/QUORRA_FEEDBACK.md` §28.7 answered this once from the zoom ladder; you
asked again, so it has been measured again with an instrument built for this question rather than
for fidelity — `crates/render-quorra/examples/viewport_refusal.rs`, which draws the gate's target
and the product's in the same run, on the real adapter, headless.

AMD Radeon 890M (RADV STRIX1), `Coverage::Cpu` — the lane `viewer-ui` uses below its
`GPU_COVERAGE_MAGNIFICATION` of ten, and the lane `REFUSED_AT_FOUR` is measured on. Window
1600 × 1000. `issue1905.pdf` page 1 is 1247 × 1984 units, so 4988 × 7936 px at 4×:

| the request | 4× | result |
|---|---|---|
| **the gate's**: whole page in one target | 4988 × 7936 | **REFUSED** — "the frame's rasterised coverage outgrew the 16384x16384 scratch image this adapter allows" |
| the product's: window, scrolled top-left | 1600 × 1000 | drawn, 38.6 % of the frame marked |
| the product's: window, centred | 1600 × 1000 | drawn, 54.5 % marked |
| the product's: window, scrolled to the far corner | 1600 × 1000 | drawn, 54.7 % marked |

**It does not begin to refuse further up, either.** At 16× the whole-page target is 19952 × 31744
and fails on the side limit before the sheet is even reached; the window frame draws. At 64× —
`viewer-core`'s `ZOOM_RANGE` maximum, so the most a person can ask for — the whole page would be
79808 × 126976 and the window frame still draws. There is no magnification this viewer permits at
which `issue1905.pdf` refuses.

`bug1703683_page2_reduced.pdf` behaves identically: refused as a whole page at 4× (and it is only
2448 × 3168, which is the interesting part — the sheet is the sum of coverage tiles and not the
frame's extent, so shrinking the frame culls the tiles), drawn in every window frame.

**The other two of `REFUSED_AT_FOUR` refuse both ways**, which is the control this measurement
needed: `bug1721218_reduced.pdf` (§11.6.6/§11.7.2, a group compositing in a four-component
blending space) and `issue18032.pdf` (§11.4.6's non-isolated knockout group) refuse the whole-page
target *and* all three window frames, identically, because they refuse before the scene is built.
So the list is two kinds of refusal wearing one name, and only one kind is a property of the
gate's target.

**Why the marked share is in the table**, since it is the thing §28.7 had to disclaim in prose:
"the device took the frame" and "the page is on it" are different claims, and this tree has been
caught by the difference before — a vello scene that overflowed a device buffer, set a flag,
stopped filling and returned `Ok(())` over a blank target. So the example counts pixels that
differ from the medium. 38.6 % is a page, not an empty success.
(`bug1703683_page2_reduced.pdf` reads 0.0–0.9 % in a window and that is the *page*: it marks 0.5 %
of itself at 1× as a whole. Reduced test cases are mostly white, which is exactly why the count
has to be read against the page's own figure rather than against a threshold.)

### What this does and does not license

It means neither page is a user-visible defect on our account, so no tiling round is owed to us
for them. It does **not** mean our gate is wrong to refuse: 4× over a whole page is harsher than
any window on purpose, and a device that stopped refusing there would be telling us something real
about the seam. And `REFUSED_AT_FOUR` stays held to equality, because a page arriving in it is
still a hole that only appears under magnification — it is now documented as a hole in *that
target* rather than in the viewer.

---

## Where the rest of this conversation lives

`doc/HAYRO_ISSUES_FOR_QUORRA.md` carries all five corrections in place, each marked where it
stands with the standard's sentence under the clause number — a hand-over document that quietly
acquired the right citation would teach nobody anything. `doc/QUORRA_FEEDBACK.md` is the standing
document; `doc/HAYRO_ISSUES.md` is the reading of the ~130 issues that are this tree's business
rather than yours.
