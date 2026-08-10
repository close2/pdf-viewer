# ADR 0263 — A right inverse of the press, and the gamut it brings with it

Date: 2026-08-10 (session 427)
Status: accepted

## Context

ADR 0262 drew §11.4.7's page group in the `DeviceCMYK` blending space it states — four components
in two rasters, no new format — and then found that the format was never the blocking half.
**Seven of 69 sampled web pages drew and 61 were held by one thing**: §11.7.2 requires a colour to
be *converted into* the blending space, and this tree had no such conversion.

> If the colour space of a graphics object within the group is not equivalent to the group's
> blending colour space, then it shall be converted to the group's colour space , and all blending
> and compositing computations shall be done in that space

ADR 0262 read §11.7.5.3's black-generation bullets as naming §10.4.2.4 for that conversion, measured
what taking it as written would cost, and **refused to ship it**: §10.4.2.1 packages §10.4.2.2
through §10.4.2.5 as what a processor uses *instead of* §10.3, ADRs 0009 and 0042 put this tree's
conversion *out* of `DeviceCMYK` on §10.3's branch, and composing one branch with the other moves a
colour the clause never asked to move. `0950007.pdf`'s green panel came back grey-green and its
`0 g` text came back the process black `#231F20`, on a page where neither mark composites with
anything.

So this round's question is not "which algorithm" but **which colour model a page uses, and where
the boundary is** — because a page drawn by two of them is the defect ADR 0262 photographed.

## The clause that decides it, and it is the paragraph before the bullets

§11.7.5.3's *first* substantive paragraph names the conversion by its **target** rather than by its
algorithm:

> The rendering intent influences the conversion from a CIE-based colour space to a target colour
> space, taking into account the target space's colour gamut (the range of colours it can
> reproduce). Whereas in the opaque imaging model the target space shall always be the native
> colour space of the output device, in the transparent model it may instead be the group colour
> space of a transparency group into which an object is being painted.

That sentence settles the round. **The conversion into a group's colour space is the same conversion
as the one onto the device, with a different target** — so it sits on whichever branch of §10.4.2.1
the processor is already on, and this tree is on §10.3's:

> Although ICC enabled PDF processors should always follow the provisions and recommendations
> provided in 10.3, "CIE-Based colour to device colour", a less-capable PDF processor may choose to
> use the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5.

The black-generation bullets ADR 0262 read are §10.4.2's side of that fork. Their preamble says so —
"[a] similar approach works for the black-generation and undercolour-removal functions, which shall
be applied only during conversion from `DeviceRGB` to `DeviceCMYK` colour spaces" — and what they
bind is *which* functions §10.4.2.4 uses when it is the route, not that it is.

**One nearby sentence was read and found not to apply**, which is worth recording because it points
the other way. §10.3.2 says: "If the native device colour space is CMYK, then converting colours in
the `DeviceGray` colour space to that CMYK should follow the method described in 10.4.2.3". That is
a `should` on §10.3's own branch, and it would send `0 g` to `[0 0 0 1]` — the process black ADR
0262 refused. Its condition is *the native device colour space*, and this device's is a screen's
three components; §11.7.5.3 substitutes a group's space for the device's in a *colour conversion*,
not in the question of what the device is. So the sentence is not in force here. Session 426's
measurement that "exempting `DeviceGray` alone closes 19 more" was therefore an offer to take,
without a clause, the one shortcut that puts black text at `#231F20`.

## Decision — the standard's ranking everywhere, which means one model and no boundary

Of the three shapes the round was set:

1. **§10.4.2.4 in, §10.4.2.5 out.** Self-consistent and exact; costs the corpus 802/88 → 800/90
   (ADR 0042) and renders process magenta as `#FF00FF`. **Rejected**: it fixes a composite by
   moving every `DeviceCMYK` pixel on every page, which is the trade ADR 0262 already priced and
   lost.
2. **§10.4.2's pair inside a departing group, §10.3's elsewhere.** **Rejected outright.** It is the
   two-model page in a smaller box, and no argument was found for why the boundary would not be the
   defect ADR 0262 photographed — the whole visible failure there is a colour crossing between two
   models on one page.
3. **A right inverse of the ink cube.** **Taken.** The conversion into `DeviceCMYK` is the inverse
   of the conversion out of it, so there is only one colour model on a page and no boundary to
   argue about. §10.3's branch says what that model *is*: `CMYK_CORNERS` stands in for a press's
   profile under §10.3.2's licence, and converting into the space it defines is asking which ink
   that press would lay down to make this colour.

`colour::rgb_to_ink` is that conversion. §10.4.2.4 does not leave the tree; it keeps two jobs inside
the search, and both are the clause's own — the nominal `k`, "the minimum of the intermediate c , m
, and y values that have been computed by subtracting the original red , green , and blue components
from 1.0", and the separation the search starts from.

### The construction

1. **The nominal separation** gives the starting point and the black generation's input.
2. **The three chromatic inks are solved** so that the cube reproduces the colour at that black —
   Gauss–Newton on the trilinear slice, projected onto the unit cube and backtracked so the squared
   distance falls at every step. This is undercolour removal *computed* rather than asked of a
   function: §10.4.2.4 wants "the amount to subtract from each of the intermediate c , m , and y
   values", and at a known press the amount is not a guess.
3. **Where no three exist, another black generation is tried**, down a ladder of twelve from all the
   black there is to none, and the first that reproduces the colour is the most black that does. The
   clause hands this to the device — "[e]ach device shall be configured with default values that are
   appropriate for that device" — and names the freedom exactly: a black-generation function "may
   simply return its k operand unchanged, or it may return a larger value for extra black, a smaller
   value for less black, or 0.0 for no black at all." **The ladder reaches above the nominal `k` as
   well as below it**, because this press's black ink is `#231F20` rather than `#000000` and a very
   dark saturated colour needs more black than the nominal.
4. **A four-ink polish** closes a colour whose feasible band of black falls between two rungs.
5. **What is left is outside the press's gamut.**

### The gamut mapping is a choice, and it is recorded as one

No clause states which colour to substitute for one the target space cannot make. §11.7.5.3 states
that the question exists — the rendering intent governs the conversion "taking into account the
target space's colour gamut (the range of colours it can reproduce)" — and ISO 15076-1 states the
answer, which is a standard this project does not hold. **The choice made here is the nearest
reachable colour by squared distance in the device's own three components**, and it is a choice
rather than a derivation. `#FF0000` is the standing example: no mixture of these inks makes it, and
it lands on `#ED1C24`, the cube's own red corner.

### It is a table, because a page is made of colours and not of fills

The search costs **12.5 µs** a colour, which a page with a photograph on it pays a million times.
So `rgb_to_ink` reads a 17 × 17 × 17 table of separations and takes up to six Gauss–Newton steps
from it; the table is the search over a grid of sRGB, built once behind a `OnceLock` and only where
a page asks for it, in **7.5–10.0 ms** across this machine's 24 threads (61.7 ms on one). Per
distinct colour that is **791 ns** on a document-like population against 12.5 µs, and the answer is
the search's own: over 800 random colours of the cube's image the worst gap after the polish is
**0.50 of 255**, which is the threshold itself.

The map is a pure function of a compile-time constant, so it could be generated data instead. It is
not, because `CLAUDE.md`'s launch rule is that anything not needed to show page one is deferred to
first use, and this is wanted by 0.6% of the corpus and 3.5% of the web.

## What the measurement says

### The two populations

| | before | after |
|---|---|---|
| **SafeDocs, 1944 documents** — incomplete | 82 | **34** |
| — naming §11.4.7's blending space | 62 | **13** |
| — of the 69 that state such a space, drawn in it | 7 | **56** |
| **pdf.js corpus, 974 documents** — incomplete | 69 | **68** |
| — of the 7 that state such a space, drawn in it | 1 | **3** |

### The pictures, which are what this round is answerable to

**All 61 web witnesses were rendered before and after at 1.5× and compared, and the biggest movers
were looked at.** Twelve are byte-identical. The largest change is `0750022.pdf` at RMSE **2298 of
65535**, half of what ADR 0262 measured for §10.4.2.4's route on the same population, and it is a
corporate flyer whose banner photograph loses saturation where the inks cannot reach it.

**`0950007.pdf` — ADR 0262's own picture — is the one to read.** Its green panel is `#007A61` before
and `#007A61` after, to the byte; its black text is black; its photograph is unchanged. That is the
right inverse's whole claim, shown rather than asserted: a colour the assumed inks can make is
separated, composited in four components and converted back to the colour the file states.

**What does move is saturated flat colour**, and `7550002.pdf` is the measurement: a Trentino course
poster whose brand panel is `srgb(235, 0, 69)` before and `srgb(233, 18, 72)` after — **18 levels in
one channel**, on a pink no process ink can print. That is the gamut, and it is the cost of the
choice recorded above.

### Against another reader

Of the 61, **46 move away from `poppler`, 3 toward it and 12 are unmoved**, and the mean distance
goes 3650.8 → 3711.3 of 65535 — 5.57% to 5.66%, a 1.7% widening of a gap that was already there.
Principle 5 decides what to do with that: `poppler` composites these pages on the device's three
components, so a page group's blending space cannot move it at all, and the clauses quoted above are
what this tree is answerable to. **The number is recorded rather than acted on**, and it is the
honest shape of the disagreement: we are further from another reader on 46 pages, by a fraction of
the distance already between us.

### The oracle, which is the instrument with a consensus behind it

**Every verdict count is identical** — 905 agreeing, 68 contradicted, 786 ambiguous, 1 our geometry,
2 reference geometry, 14 not comparable, 18 no render — and **one line of 1794 differs**:
`issue12798_page1_reduced.pdf` page 1 loses its `(incomplete)` marker. That page is a Dutch
public-health poster composited in ink now, and it is the corroboration the SafeDocs numbers cannot
give:

```text
             72 dpi   288 dpi   576 dpi
poppler     23.7199   23.8327   23.8408
mupdf       23.8604   23.9477   23.9610
ours        23.8002   23.8373   23.8438
```

**Our row is what `AMBIGUOUS_NON_ISOLATED_POSTER` recorded in the four-hundredth session, to the
fourth decimal**, measured after the page started compositing in four components. The band's colour
is `#E60575` against `poppler`'s `#E60576`, `mupdf`'s `#E60376` and `ghostscript`'s `#E50275`. What
moved is the worst *tile*, 8.79 → 9.14, and the difference image says where: the outlines of two
lines of small white type and the band's top edge, with the band's interior black.

### What it costs to draw

Page one of all 61 witnesses at 1.5×, wall clock: **18.07 s → 27.95 s**. Of the 9.9 s, **8.0 s is
ADR 0262's second interpretation and second rasterisation** — measured by putting the cheap
conversion back and leaving the two passes — and **1.8 s is this round's conversion**, against
**37.3 s** when every colour ran the search. The worst single page is `6750011.pdf` at 1.36 → 3.51 s.

### The rest of the gates

- **tests 1558 → 1561**, 11 skipped; **conformance 6317 → 6345 citations**, 592 quotations unmoved,
  875 ledger rows with `reported` 19 → **18** and `partial` 251 → **252**.
- **oracle 1689 → 1690 complete, 105 → 104 incomplete**, verdict counts identical.
- **quorra 911/36/10/17 → 910/36/11/17.** The eleventh refusal is `bug1365930.pdf` and it is
  re-ratcheted with its argument: that page states `/CS /DeviceCMYK` and reports *nothing*, because
  nothing on it composites — it drew on the device's three components and quorra agreed. It takes
  §11.4.7's pair of rasters now, because "[a]ll page-level compositing shall be done in the default
  blending colour space of the page" is not conditioned on two marks overlapping, and the backend
  renders one raster. `QUORRA_FEEDBACK.md` section 17 answers it along with the tenth.
- **text 99.2% (24003/24187) unmoved** with 66 → 65 documents ungated; PDFBox 99.8% (14257/14281),
  dates 1514 of 1545, XMP 318 read with 3191 properties, JPEG 2000 14 byte-identical — all unmoved.
- **`doc/todo/00`'s step 7 over all 786 ambiguous pages, run before and after: one line differs**,
  `issue12798_page1_reduced.pdf` +0.068 `[incomplete]` → **+0.080**. Twenty at or past −1 and sixteen
  of them incomplete, head `issue16038.pdf` −5.758, `issue12295.pdf` −1.712,
  `checkbox_no_appearance.pdf` −1.200, `issue14297.pdf` −1.145, `issue7821.pdf` −1.000 — the same
  five names in the same order as the three-hundred-and-ninety-seventh's run, the alarm holding for
  the thirteenth consecutive time.

## What the round found on the way, and reported rather than drew

**A document that says what its own `DeviceCMYK` is.** §8.6.5.6's `/DefaultCMYK` — "[i]f such an
entry is present, its value shall be used as the colour space for the operation currently being
performed" — and §14.11.5's output intent both outrank the assumed process inks, and this tree
honours both for a colour on its way to a pixel (ADR 0009). On a page composited in ink they decide
something more: the four components §11.3.4 composites are *that* press's, and compositing in ours
is a different picture. The colour still reaches the right screen pixel, because the conversion in is
the inverse of the conversion out whichever press is assumed; what is wrong is a composite. **Seven
of the 1944 web documents are in that position and none of the 974**, and they are named rather than
drawn. Trap 5's rule, applied to a silence this round would otherwise have created.

## What is left, and where it is written

`doc/todo/23` carries five conditions, each a different clause and each named per page: the press
above, an `ICCBased` four-component blending space, a group inside the page composing in another
space (§11.6.6), Table 57's black generation (§11.7.5.3), and a non-separable blend mode
(§11.3.5.3). Thirteen of 1944 web documents and four of 974 corpus documents, against 62 and 5.

## The lesson, and it is about where a branch is chosen

ADR 0262 read §11.7.5.3's bullets, found §10.4.2.4 named in them, and concluded the standard had
chosen the algorithm. It had not: **the paragraph above the bullets chooses a *target*, and the
algorithm follows from the branch the processor is already on.** Reading a clause from its
subheading down is how a conditional sentence becomes an unconditional one, and the cost of getting
it wrong was visible — a green panel gone grey in a picture nobody would have looked at if the
counts had been the only instrument.

The second lesson is smaller and older: **an exact answer that is 12.5 µs is not an answer.** The
construction here is the search *and* the table, and the table is not an approximation of the search
— it is where the search's answer is kept.
