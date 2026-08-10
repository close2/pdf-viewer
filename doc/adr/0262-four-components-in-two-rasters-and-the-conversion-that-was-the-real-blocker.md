# ADR 0262 — Four components in two rasters, and the conversion that was the real blocker

Date: 2026-08-10 (session 426)
Status: accepted

## Context

`doc/todo/23` had one population left and session 425 measured it against the world: of 1896
documents sampled from the web through SafeDocs, **67 of the 86 reports were §11.4.7's page-group
blending space** — 3.5% of real files against 0.7% of the pdf.js corpus's 974. The largest gap this
tree has against real files, by six times over everything else together.

ADR 0251 priced it and the price was a *format*: "what is owed is a four-component raster per
group … and three backends taught the format". It also established, correctly, that the number of
components is irrelevant to whether the two orders of operation agree — §11.3.3 under `Normal` is a
weighted average, so compositing in the group's space and compositing in the device's agree exactly
when the conversion out is affine, this tree's multilinear ink cube is affine on no face of the
cube, and the gap is 48 of 255 at worst and 51.5 at half of registration black over paper.

The prompt for this round named three routes: build the four-component raster, take ADR 0220's
trick and defer §10.4.2.5's clamp, or find something the clause permits that neither assumes. It is
the third, and then the round found that the four components were never the hard part.

## Decision 1 — four components fit in two rasters, because the formula is per component

§11.3.4 states the compositing formula's shape, and the sentence is the whole construction:

> The i th component of the result colour 𝐶𝑟 shall be obtained by applying the compositing formula
> to the i th components of the constituent colours 𝐶𝑏 , Cs , and 𝐵(𝐶𝑏,𝐶𝑠) .

A rasteriser that composites three channels therefore composites four if it is run **twice on the
same geometry with a different three loaded**. §11.3.5.2's separable blend functions are per
component as well, so every blend mode a page can state except four comes along unchanged. And
§11.3.4 asks for the components in additive form —

> When performing blending operations in subtractive colour spaces ( DeviceCMYK , ICCBased 'CMYK',
> Separation , and DeviceN ), the colour component values shall be complemented (subtracted from
> 1.0) before the blend function is applied and the results of the function shall then be
> complemented back before being used.

— which is met by *storing* the complements rather than by an arithmetic step around every blend.

So: `pdf_model::interpret_with` interprets a page whose §11.4.7 group states `/CS /DeviceCMYK`
**twice**, once with `Compositing::Subtractive(Half::Chromatic)` carrying `1−c`, `1−m`, `1−y` and
once with `Half::Black` carrying `1−k` in all three channels. The display list carries the second
list beside the first and the conversion out as sixteen numbers — `pdf_render::BlendingSpace`, the
ink cube's own corners, so a backend still never sees a colour space. `render-cpu` rasterises both
and `pdf_render::blending::resolve` puts them together **before** `impose_on_medium`, which is
exactly where §11.4.7 puts the conversion: "the entire result shall then … be converted to the
native colour space of the output device before being composited with the context-dependent
backdrop."

What that costs, stated rather than hidden:

- **Two interpretations and two rasterisations** for a page that states such a group — 3.5% of the
  web sample, 0.6% of the corpus. Nothing else pays anything.
- **One level of 255**, and it is the same argument ADR 0220 made for a mask's channel. The
  components are recovered by dividing the premultiplied channel by the alpha, which resolves a
  component to `1 ÷ 255α`; the conversion is a convex combination of colours in `0..=1` and cannot
  magnify an error; and the result is multiplied by `α` again, so the error in the written pixel is
  bounded by `α × 1 ÷ 255α` — one level, whatever the alpha was.
- **`render-gpu` and `render-quorra` refuse the list by name.** A Vello layer and a
  `quorra_scene::Scene` each render one raster. The GPU refusal sends the frame to the CPU backend,
  which is the job `CLAUDE.md` keeps that backend for; quorra's is `QUORRA_FEEDBACK.md` section 17,
  and it is the smallest request that file carries — no new scene vocabulary, only a way to run the
  pipeline twice and hand back both readbacks.

A **geometry digest** guards the pair. The two interpretations differ only in what a colour
resolves to, so their commands, clips, masks, blend modes and nesting are identical by
construction; `DisplayList::geometry_digest` is what checks it, because the halves are put together
per pixel and a command present in one and absent from the other would be composited against a
shape that never drew it. A mismatch falls back to the device's components and the report.

**Route 2 was not taken and the arithmetic says why.** Deferring §10.4.2.5's clamp does make the
composite exact — ADR 0251 measured 3.4 × 10⁻¹⁶ — but it replaces the conversion out with
§10.4.2.5's classic formula, which renders process magenta as `#FF00FF` and is off by up to 115 of
255 at the cube's corners (ADR 0009 measured the whole corpus: 802 agreeing and 88 contradicted
become 800 and 90). It would fix an error of at most 48 of 255 at composited pixels by introducing
one of up to 115 at every `DeviceCMYK` pixel. That is a trade, and it loses.

## Decision 2 — what actually blocks the population is the conversion *into* the space

With the two passes built, 59 of the 69 sampled witnesses drew in their blending space. **Then the
pages were looked at**, which is trap 1, and the picture is the finding of the round.
`0950007.pdf` is a Spanish ministry's webinar flyer: its green panel came out grey-green and its
black text came out `#231F20`.

Neither of those marks composites with anything. They changed because §11.7.2 requires a colour to
be *converted into* the blending space —

> If the colour space of a graphics object within the group is not equivalent to the group's
> blending colour space, then it shall be converted to the group's colour space , and all blending
> and compositing computations shall be done in that space

— and §11.7.5.3 names the route:

> When painting an elementary object with a DeviceRGB colour directly into a transparency group
> whose colour space is DeviceCMYK , the functions used shall be the current black-generation and
> undercolour-removal functions in effect in the graphics state at the time of the painting
> operation.

Those functions are §10.4.2.4's, and §10.4.2.1 packages §10.4.2.2 through §10.4.2.5 as what a
processor uses **instead of** §10.3: "[a]lthough ICC enabled PDF processors should always follow the
provisions and recommendations provided in 10.3 … a less-capable PDF processor may choose to use the
algorithms specified in the following subclauses". ADRs 0009 and 0042 put this tree's conversion
*out* of `DeviceCMYK` on §10.3's branch, by assuming standard process inks under §10.3.2's licence.

**Composing one branch with the other is not the identity, and the two halves of the standard's own
package are.** §10.4.2.4 with the nominal functions gives `cyan = c − k` and `black = k`, so
§10.4.2.5's `1 − min(1, cyan + black)` is `1 − c`, which is `red` — exact, for every colour, which
is why a processor on that branch pays nothing for the round trip and is a fact worth having in a
test (`the_conversion_into_ink_round_trips_through_the_classic_formula_and_not_the_cube`). §10.4.2.4
followed by the *ink cube* is not: pure red comes back `#ED1C24`, `0 g` comes back `#231F20`, and a
saturated green comes back grey-green because `UCR(k) = k` is the most extreme undercolour removal
there is and the cube's black axis is a straight line to process black.

So the round did not ship that page. **A page is drawn in its blending space only where every
colour painted into it is already in that space** — `ColourSpace::is_subtractive`, recursing through
`Indexed`, `Separation` and `DeviceN` to the alternate §11.3.4 says a spot colour reverts to — and a
page that paints anything else keeps §11.4.7's report with the reason in it. The residue is a
*named missing piece*, not a narrowed condition: what is missing is a conversion into the blending
space on §10.3's branch, which is the inverse of the press this tree assumes and is a colour
management round of its own.

Four more conditions are named the same way, each a different clause asking for something the pair
of rasters does not carry:

| what | witnesses of 69 |
|---|---|
| a colour from outside the blending space (§11.7.2) | 61 |
| an `ICCBased` four-component space, whose conversion out is a profile rather than sixteen corners | 6 of those, and 6 alone |
| a group inside the page composites in a different space (§11.6.6) | 3 |
| a non-separable blend mode: §11.3.5.3 gives the black component a rule of its own | 2 |
| an `/ExtGState` states Table 57's `/BG`, `/BG2`, `/UCR` or `/UCR2` (§11.7.5.3) | 1 |

The last of those was **silent** before this round: a document stating its own black generation was
drawn with the defaults and nothing said so.

## Consequences

### The measurement, and it was taken three ways

**The population, before and after, over the 1944 SafeDocs documents now in `corpus-cache/`:**
89 incomplete with 69 naming the blending space → **82 incomplete with 62 naming it**. Seven pages
drawn in ink: `1050022`, `2150021`, `3250021`, `4350004`, `5150000`, `6050016`, `6250005`. No
document became incomplete that was not.

**The counterfactual was measured rather than argued.** With the §11.7.2 condition removed — every
foreign colour taken through §10.4.2.4 — 59 of the 69 close and every one of the 69 witnesses was
rendered before and after at 1.5×: 59 pages move, 10 do not, and the ten are exactly the ten that
still report, which is the consistency check. The largest mover is `0950007.pdf` at RMSE 5295 of
65535, and it is the page above. With only `DeviceGray` exempted the count is 43 rather than 62, so
grey alone accounts for 19 of the 55 blocked pages — recorded because it is where the next round
will be tempted, and taking it would be assuming the answer rather than deriving it.

**On the seven that close, the change is small and in the right direction**, which is what a page
whose foreign colours are absent should show: RMSE 17 to 547 of 65535, and at 460 pixels wide the
before and after of `5150000.pdf` — a LEGO instruction sheet — are indistinguishable. Only the
composited pixels moved.

**The fixture is the clause's own arithmetic.** `a_page_group_in_ink_composites_in_ink` paints paper
as `0 0 0 0 k` and half of registration black over it: composited in ink that is `[0.5, 0.5, 0.5,
0.5]`, the average of the cube's sixteen corners and **76 of 255** in red, against **127** for the
average of black and white. The old route is put back in the same test by a page that states no
`/CS` at all, which gets 127 — so the number asserted is a difference this round made rather than a
number that was already there. The backdrop has to be *painted*: §11.4.7's page group is isolated,
so an unpainted pixel is transparent and the medium is composited after the conversion, where both
orders of operation agree.

### What moved on the gates

Re-run before and after; this round moves pixels.

- **corpus 974 with 70 → 69 incomplete.** `personwithdog.pdf` leaves; the other five keep the
  report with its reason attached.
- **oracle 1794 pages, 1688 → 1689 complete and 106 → 105 incomplete**, with every verdict count
  identical — agrees **905** (861 complete), contradicted **68** (66), ambiguous **786**
  (750 → **751** complete), our geometry 1, reference geometry 2, not comparable 14, no render 18.
  `personwithdog.pdf` page 1 returns to `AMBIGUOUS_NEAREST_THE_GEOMETRY`, the group it left in the
  four-hundred-and-fifteenth when it started reporting, and its ladder moved by exactly what ADR
  0251 predicts: **21.620 → 21.720** at the page's own scale and 21.722 → 21.822 at 576 dpi, +0.100
  of 255 at both, still inside the 21.991 / 21.101 bracket the two references leave.
- **quorra 957 pages: 912 / 36 / 9 / 17 → 911 agree, 36 differ, 10 refused, 17 not comparable.**
  The tenth refusal is `personwithdog.pdf` and it replaces a *report* rather than an agreement.
- **text 99.2% (24003/24187 words) unmoved with 67 → 66 documents ungated**, 23 below 90% unmoved;
  the PDFBox gate 99.8% (14257/14281) unmoved.
- **dates 1514 of 1545, XMP 318 read and 1 refused with 3191 properties, JPEG 2000 14
  byte-identical** — all unmoved.
- **conformance 6240 → 6317 citations, 584 → 592 quotations**, 875 ledger rows with all six status
  counts unmoved (401 / 251 / 19 / 83 / 8 / 113). **workspace tests 1550 → 1558**, 11 skipped.
- **`doc/todo/00`'s step 7 over all 786 ambiguous pages, run before and after: one line of 786
  differs.** `personwithdog.pdf` +0.719 `[incomplete]` → **+0.819**, which is the same +0.100 of
  ink the ladder measured and a label that lost its bracket. Twenty at or past −1 and sixteen of
  them incomplete, head `issue16038.pdf` −5.758, `issue12295.pdf` −1.712,
  `checkbox_no_appearance.pdf` −1.200, `issue14297.pdf` −1.146, `issue7821.pdf` −1.000 — the same
  five names in the same order to the thousandth as the three-hundred-and-ninety-seventh's,
  four-hundred-and-sixth's and four-hundred-and-fifteenth's runs, the alarm holding for the twelfth
  consecutive time.

### The lesson, and it is about which half of a requirement was priced

Every one of the five transparency rounds before this one found a clause whose arithmetic was
cheaper than the tree assumed. This one found that too — the four components are two rasters and
the display list needed one field — but the more useful finding is that **the priced half was not
the blocking half**. ADR 0251 measured the compositing and concluded a raster format; the format
took an afternoon and closed seven pages, and the sixty-one that stayed shut are held by a *colour
conversion* nobody had counted, in a clause (§11.7.2) the tree had read for its inheritance rule
and not for its second sentence.

The instrument that said so was not a gate. It was rendering sixty-nine real pages before and after
and looking at the biggest mover, which is trap 1's rule and the only reason this round did not
ship a page whose green panel had gone grey.
