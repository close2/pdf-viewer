# Transparency departures

Status: each reported where it can change a pixel. **§11.5.3's population is closed** — its device
branch was taken in the three-hundred-and-eightieth session (ADR 0217) and both residues in the
three-hundred-and-eighty-third (ADR 0220) — **§11.4.6's shape is closed in the
three-hundred-and-ninety-seventh** (ADR 0234), and **§11.4.4's non-isolated group is drawn since
the four-hundredth** (ADR 0237). The four-hundred-and-fifteenth found the standing population was
the wrong one and priced what is left of it (ADR 0251); **the four-hundred-and-twenty-sixth built
what it priced and found the price was for the wrong half** (ADR 0262); **the
four-hundred-and-twenty-seventh built that other half and closed the standing item** (ADR 0263);
**the four-hundred-and-thirty-sixth made the press the document's** and closed the largest
condition the web has (ADR 0272). **The four-hundred-and-thirty-eighth took the first of the two
backend rows off this file** (ADR 0274): `render-quorra` draws §11.4.4's non-isolated group, and
the two rows still on it — §11.4.6's stated shape and §11.4.7's two rasters — stopped being
requests to somebody else and became work here, because quorra answered both asks at `89d7dd77`.
**The four-hundred-and-thirty-ninth did one of the two and found the other is still not writable**
(ADR 0275): §11.4.7's two rasters are drawn through quorra and three corpus pages left the refused
list, while §11.4.6's two marks cannot be *asked for* at `89d7dd77` — the operators exist and the
one position this tree emits them from is one of the two the builder refuses. That row stays, with
its reason corrected and an ask written (`doc/QUORRA_FEEDBACK.md` section 14.2). **The
four-hundred-and-fifty-sixth took the second backend row off this file** (ADR 0291): quorra lifted
both refusals at `2c9bdd0`, `render-quorra` states §11.4.6's two stages, and the four corpus pages
that were refused for it agree with the CPU oracle — so no backend row is left here and what
remains is the interpreter's. **The
four-hundred-and-fortieth asked the standing row what it contained and 77 of its 85 were a soft
mask** (ADR 0276): the flag that says a group changed the page's blending space was not scoped to
the page the way the space itself is, so an isolated group inside an `/SMask`'s group took the whole
page off §11.4.7's ink route. Corpus 68 → 65 incomplete, web 905 → 851, and this row is 85 → 8.
**The four-hundred-and-forty-first closed §11.3.5.3's row with no code written for it at all**
(ADR 0277): the rule that clause gives the black component is what its own four functions return on
the neutral colour the black raster holds, so a backend that implements §11.3.5.3 for three
components implements it for four. Web 851 → **824** incomplete — 27 of the 31 complete, 4 keeping
§11.4.4's report — that row 31 → 0, and the refusal it fired from
was deleted rather than narrowed.
**The four-hundred-and-seventy-second read §11.4.6 for *which* backdrop a knockout group hands
each element and found this file's second open item was two documents that needed no construction
at all** (ADR 0307): a knockout group whose rule can change no pixel is §11.4.4's group, and
NOTE 6 makes a non-isolated group nested in an isolated knockout group §11.4.5's. Corpus 67 → 65
incomplete and the oracle's contradicted set 68 → 67: `knockout_blend_multiply.pdf` draws the
colour §11.3.5.2 states rather than one two channels away, which all three references were
already drawing, and `knockout_inner_backdrop.pdf` — which was drawn right all along — stops
reporting a departure it does not have. **The four-hundred-and-ninety-second built both of this
file's last two constructions** (ADR 0327): §11.4.6's non-isolated knockout group is drawn on the
oracle with the initial backdrop retained beside the accumulation, and an isolated group that
*introduces* a sampleable four-component space composites in it as §11.4.7's pair one scope down
— `Command::Group` carries the space and the second element list, which is word for word what
this file priced. Corpus 65 → 63 incomplete: `issue18032.pdf` **agrees with the CPU oracle** now
(its `/AIS` blocker was a page-wide flag a `Q`-restored statement two forms away had set, scoped
to the group in the same round), and `bug1721218_reduced.pdf` is drawn in ink and joins
`AMBIGUOUS_PAGE_DRAWN_IN_INK` with its own reading — nearer to `poppler` than any two references
sit to each other. Both are refused by name on `render-gpu` and `render-quorra` and the frames go
to the oracle, which is a backend row again only in the sense ADR 0327 prices: a scene under
composition cannot resolve a pair per pixel or retain a backdrop beside a layer.
**The five-hundred-and-eightieth honoured §11.6.4.3's `/AIS`** (ADR 0415), which had been read and
refused since ADR 0234 and which this file had twice priced as "a second `stated_shape`": under
`/AIS true` §11.3.7.2's three opacity inputs are all 1.0 — §11.6.4.2 gives the object's, this
clause and §11.6.4.4 give the other two away — so the alpha a rasteriser already draws an element
with *is* its shape, and the shape half of `Command::Shaped` is the element itself. No vocabulary,
no backend arm, no raster. The refusal was **inverted**: `/AIS true` is the one reading under which
one number per pixel cannot disagree with the shape. Corpus: 0 pages, measured over every page of
the nine documents that state the entry. Web: 1 of 65 944, `6573550.pdf`, whose knockout group
draws now.
Priority: 23
Corpus: 0 documents
Clauses: §11.3.5.3, §11.3.7.2, §11.4.4, §11.4.6, §11.5.3, §11.6.4.3, §11.6.4.4, §11.6.6, §11.7.5.3,
§8.6.5.5, §8.6.5.6, §8.6.5.7, §11.7.2, §14.11.5
Code: `crates/pdf-model/src/content/transparency.rs`, `crates/pdf-model/src/colour.rs`,
`crates/pdf-render/src/blending.rs`, `crates/pdf-render/src/display_list.rs`,
`crates/render-cpu/src/lib.rs`

| | corpus | web witnesses | what it is |
|---|---|---|---|
| ~~a non-separable blend mode on such a page (§11.3.5.3)~~ | ~~1~~ → 0 | ~~1 of 1896, 2 of 4000, 27, 28, 31~~ → **0** | **closed in the 441st, ADR 0277: the K rule is the clause's own four functions on a neutral pair, which is what the black raster is.** No display-list member, no backend arm, no refusal — the collapse went further than the round set out to take it, and the explicit route it replaced (a `Backdrop` blend function, which is Destination-Over exactly) would have cost the quorra backend all 31 |
| a group inside the page composites in a different space (§11.6.6) — **the standing item now** | 0 | 78, 85 → **8 of 65 944** | 77 of the 85 were a mask's group counted as the page's (ADR 0276). A further **30** — 1 in the corpus, `bug1721218_reduced.pdf` — were a group that *introduces* a space on a page that states none, and **the four-hundred-and-ninety-second draws that shape** where the space is four components this tree can sample (ADR 0327): the corpus witness composites in ink. What the condition still fires on is a space the group-scoped pair cannot carry — a three- or one-component group inside a four-component parent (a per-pixel conversion between two presses), four components no profile backs, §11.7.5.3's black generation — each still reported by name where it composites. **Narrowed by §11.7.2 in the eight-hundred-and-seventy-ninth** (ADR 0797): a device-space group of the same count inside a CIE-based one *is* that space and changes nothing, so a `/DeviceCMYK` group inside a profile's press and a `/DeviceRGB` one inside an sRGB page left this condition |
| an `/ExtGState` states `/BG`, `/BG2`, `/UCR` or `/UCR2` (§11.7.5.3) | 0 | 1 of 1896, 0 of 4000, 7 → **9 of 65 944** | **was silent until the 426th**, and 0 of 4000 could have been read as noise. **All nine state it at `soft_mask_depth` 0**, measured in the 440th, so the monotone flag costs nothing here |
| a page group whose components are not four this tree can sample | 0 | 14 of 4000, 106 → **5 of 65 944** | what is left after ADR 0272: a `/DeviceGray` or `Lab` page group, or four components with no profile behind them, so §11.3.4 has no formula to apply and no conversion out. **`/DeviceGray` left this row in the eight-hundred-and-sixty-fifth** (ADR 0790): one component is three equal channels, drawn by one interpretation under `Compositing::Grey`. **`CalGray` and a one-component profile left it in the eight-hundred-and-seventy-first** (ADR 0792): the component is composited and a sampled curve out rides on the display list. **`CalRGB` and a bi-directional three-component profile never sat on this row and are drawn since the eight-hundred-and-seventy-ninth** (ADR 0797). What stays is `Lab`, which the clause forbids, a one-component space the clause does not list (`Separation`, `Indexed`), a profile whose curve has no inverse and a three-component table profile with no `B2A` — reported on every mark or where something composites, and the population of that is a number the corpus gate prints. **The nine-hundred-and-fourth drew the table profile where it has a `B2A`, took §11.5.3's mask branch with it, and turned three silences into reports** (ADR 0851); **the nine-hundred-and-seventh took the last shape §11.5.3 had left — a four-component profile as a *mask* group's `/CS`, which is §11.4.7's pair of rasters inside the mask** (ADRs 0856, 0857), so what is reported of a four-component space is now a profile §11.3.4 rules out or a budget rather than a construction nobody had built. *What the two condition rows still fire on* below says, clause by clause, which of what is left is a debt and which is this reader telling the truth about a file |
| ~~the document names the press its `DeviceCMYK` is~~ | ~~0~~ | ~~151~~ → **0** | **closed in the 436th, ADR 0272: the press is a value, and `CMYK_CORNERS` is one of them** |
| ~~a conversion *into* the blending space~~ | ~~5~~ → 0 | ~~61~~ → 0 | **closed in the 427th, ADR 0263: a right inverse of the ink cube** |
| ~~the four components themselves~~ | — | — | **closed in the 426th, ADR 0262: two rasters, no new format** |
| ~~a non-isolated group NOTE 5 cannot flatten~~ | ~~6~~ → ~~3~~ → ~~1~~ → 0 | | **the non-knockout ones closed** in the 400th, ADR 0237; **two of the three knockout ones closed in the 472nd, ADR 0307** — one was §11.4.4's group wearing `/K`, the other was §11.4.5's wearing `/I false` under NOTE 6 — and **the last, `issue18032.pdf`, closed in the 492nd, ADR 0327**: the initial backdrop is retained beside the accumulation on the oracle, per element, which is exactly the construction ADR 0307 priced |
| ~~a soft-mask group with such a space~~ | ~~7~~ → 0 | | **closed** in the 380th and 383rd, ADRs 0217 and 0220 |
| ~~a knockout element whose shape is not its coverage~~ | ~~5~~ → 0 | | **closed** in the 397th, ADR 0234 |

The rows that grew by one or two in the four-hundred-and-thirty-sixth are documents that were
reported for the press and are now reported for the next condition they meet — the population
narrowing honestly rather than a condition being narrowed (trap 5).

Each remaining one is refused *by name* rather than approximated, and since the four-hundred-and-
twenty-sixth the name says **which** of the conditions fired. No corpus document is on this file
any more: `bug1721218_reduced` and `issue18032` left in the four-hundred-and-ninety-second, both
drawn by the oracle. `knockout_blend_multiply` and `knockout_inner_backdrop` left in the
four-hundred-and-seventy-second.
`personwithdog.pdf` left in the four-hundred-and-twenty-sixth, `issue12798_page1_reduced.pdf` and
`bug1365930.pdf` in the four-hundred-and-twenty-seventh, and `bug1703683_page2_reduced.pdf`,
`bug1755507.pdf` and `issue13520.pdf` in the four-hundred-and-fortieth, each of which drew them.
`issue18032.pdf` lost its **third** report in the four-hundred-and-forty-first and keeps the two
§11.4.4 and §11.4.6 give it, which is why the corpus count did not move while the picture did.

## What the two condition rows still fire on, and how much of it is a debt

The nine-hundred-and-fourth session was asked to say, for each of them, whether it should stay
reported or become drawn, **and to derive that from the clause rather than from what would be
nice** (ADR 0850). The answer is that they are two different kinds of thing and the rows had not
told them apart:

| what fires | the clause | so |
|---|---|---|
| `Lab` as a blending or mask group space | §11.3.4: it "shall not be used as blending colour spaces because the compositing computations in such spaces do not give meaningful results when applied separately to each component" | **stays reported, for good.** A file stating one is outside what the clause admits; drawing it would be inventing an arithmetic the standard says has no meaning |
| a one-component space §11.3.4 does not list — `Separation`, `Indexed` | §11.6.6's restrictions "exclude `Lab` and lightness-chromaticity `ICCBased` colour spaces, as well as the special colour spaces `Pattern`, `Separation`, and `DeviceN`" | **stays reported, for good.** Same shape: the file is what departs |
| a profile with no way in — no `B2A` this crate reads, or a curve with no inverse | §11.3.4: "the ICC profile shall be capable of both device to PCS and PCS to device transformations" | **stays reported, for good.** §11.6.5.1 makes the `/CS` "the colour space in which the compositing computation is to be performed" and there is nothing to convert the group's marks into |
| ~~a four-component profile as a *mask* group's `/CS`~~ | §11.5.3's colorimetric branch, §11.4.7's per-component compositing | **paid in the nine-hundred-and-seventh** (ADRs 0856, 0857). It was the one row of the six that named a construction rather than a condition the file fails, and the construction was buildable: the mask group is the pair of rasters and the `Y` is the press's own over four axes. What is left on the row is the *budget* — a page that has already named `colour::MAX_PRESSES` distinct presses — which is a bound rather than a debt |
| a space the group-scoped pair cannot carry — a three- or one-component group inside a four-component parent, or two different spaces nested | §11.6.6 puts a conversion at the `Do` | **a debt.** It needs a *group-scoped* conversion between two spaces per pixel, which is a second cube keyed to the parent's space rather than a refusal that has outlived its reason |
| four components no profile backs, and §11.7.5.3's black generation | — | **a debt** and **inapplicable-by-file** respectively; unchanged |

**Three of the six are not owed work at all**, a fourth is paid,, and that is worth saying plainly because a
`reported` row reads like a debt whether or not it is one: each of the first three names a
condition the *document* fails, and the report is this reader telling the truth about a file
rather than an admission about itself. The rows above keep them because a report with no
population is still a report, and because trap 5's rule is that unsupported input stays loud —
not because there is a construction waiting to be built.

## The K rule was the RGB rule, evaluated where the K is

**§11.3.5.3 gives the black component of a subtractive blending space a rule of its own** — "[f]or
the K component, the result shall be the K component of Cb for the Hue , Saturation , and Color
blend modes; it shall be the K component of Cs for the Luminosity blend mode" — and this file, the
ledger and the interpreter all called it a blend function neither raster has. **It is not a blend
function at all** (ADR 0277).

The clause's auxiliary functions "operate on colours that are assumed to have red, green, and blue
components", which is three, which is what each raster holds. The chromatic raster holds the
complements the clause's *first* bullet asks for, so `Hue` there is already the clause. The black
raster is **neutral in all three of its channels**, and on a neutral pair `Sat` is 0, `SetSat(C, 0)`
takes the clause's own `else` arm, `SetLum` returns the neutral colour of the luminosity it is
given, and `Lum` of a neutral colour is its level — so `Hue`, `Saturation` and `Color` come to `Cb`
and `Luminosity` to `Cs`. Worst gap over 200 000 neutral pairs: **1.19 × 10⁻⁷**, one ulp, which is
the residue of `0.3 + 0.59 + 0.11` not being exactly 1 in binary floating point.

So the round deleted a refusal and added no vocabulary. The explicit alternative was written first
and withdrawn with its own arithmetic recorded: `B(Cb, Cs) = Cb` under §11.3.3 is Porter-Duff
**Destination-Over** exactly, `tiny-skia` and `peniko` both have that operator and
`quorra_scene::Compose` does not — so stating the rule rather than deriving it would have cost the
quorra backend all 31 documents and bought no pixel.

**What is now load-bearing is that the black raster is neutral**, which `Half::Black` guarantees by
painting `Color::grey`. A future change that put anything else in its other two channels would
break §11.3.5.3 silently, which is why the identity has a test on both a CPU and a GPU backend
rather than a paragraph.

## The report was the mask's, and the flag was not scoped

**77 of the 85 web documents this file called §11.6.6's standing item, and all three of the
corpus's, were a group declared inside a soft mask** (ADR 0276). `build_soft_mask` clears the
blending space in force for a mask group's content — ADR 0220's finding, because §11.5.3 reduces
such a group to one luminosity and §10.4.2.3's conversion to it is linear in the components — and
`Interpreter::blending_changed`, added later for a different question, was outside that scope. So
every isolated group inside an `/SMask`'s group compared its space against `None` and counted as a
group *the page* composites in, which took the whole page off §11.4.7's ink route.

The instrument was one `eprintln!` at the place the flag is set, printing the two spaces and the
interpreter's `soft_mask_depth`, over the 85 documents and their 1320 changes. Not one document had
changes in both places. **What the 8 real ones state** is three shapes over 32 sites: an isolated
group with a three-component `/CS` inside a `/DeviceCMYK` page (20), an isolated `/DeviceCMYK` group
inside an isolated three-component one (10), and an isolated `/DeviceGray` group inside a
`/DeviceCMYK` page (2).

**What that row needs is now one construction rather than four.** A group on the page that
introduces a space of its own has to composite its elements in that space and convert once at its
`Do`, which is ADR 0262's pair of rasters one scope down: `Command::Group` would carry a blending
space and a second command list, and three backends would have to resolve the pair before
compositing the group onto its parent. Both halves of the conversion between two presses already
exist — `Press::blending_space` out and `colour::rgb_to_ink` in — so what is missing is the display
list's vocabulary and not the arithmetic. 8 web documents and 0 corpus ones is what it is worth.
**Built in the four-hundred-and-ninety-second for the four-component shape** (ADR 0327):
`pdf_render::GroupBlending` is that vocabulary word for word, the oracle resolves the pair, and
the other two backends refuse it by name. What the row keeps is the *other* direction — a three-
or one-component group inside a four-component parent, whose conversion out lands in the parent's
ink per pixel, which is a conversion between two presses no sampled grid here expresses — plus
four components no profile backs and §11.7.5.3's stated black generation. **The one-component
group on a page compositing on the device is drawn since the eight-hundred-and-sixty-fifth** (ADR
0790, `Interpreter::group_grey`): its result is grey in every channel and §10.4.2.2's conversion
out is the identity on that, so it composites onto its parent as any group does. The one-component
group *inside a press* is recorded whatever it holds now — a one-component conversion changes an
opaque mark — so the pair falls back, the group is drawn grey on the device and the press is
reported; that is louder than the ink it was drawn in before, and it is the shape a per-pixel
conversion between a grey and a press would close.

**54 of the 77 become complete and 23 keep a report they already had** — 21 of §11.4.4's
non-isolated group, one knockout, and three that join the non-separable row above. Web blending
reports over all 65 944: **157 over 156 documents → 83 over 82**.

## The press is the document's, and no ICC dependency was needed

**292 of the 65 703 web documents that open name a press**, and 286 of those name four components:
186 by a page group `/CS` that is a four-component `ICCBased` space (§11.7.2), 94 by §14.11.5's
output intent, 6 by §8.6.5.6's `/DefaultCMYK`. **Every one of those 286 profiles parses with the
`A2B` evaluator ADR 0009 wrote in this tree**, so the round that was set up as a dependency
question was a reading one. `crates/pdf-model/examples/press_census.rs` is the instrument.

Two clauses decide the direction. §8.6.5.5 requires the *file* to carry `B2A` for a blending-space
profile — and all 286 do — but places no requirement on the processor; §14.11.5's Table 401 names
`A2B` for this device outright: "the 'to CIE' (AToB) information may optionally be used to remap
source colour values to some other destination colour space, such as for screen preview or
hardcopy proofing". A screen is what this processor has. The conversion *into* the press is then
ADR 0263's right inverse of the same sampling, so a page has one colour model and no boundary.

**The residue this created is a number and it is the thing to watch.** A backend interpolates a
table, so a press is sampled onto a grid of seventeen per axis; over the 286 profiles that grid
departs from evaluating the profile by a median 5.99 and at most 14.52 of 255. No feasible side
reaches half a level — a v2 CMYK profile puts a steep sampled curve on each ink *before* its own
table, and sampling in linear light is worse rather than better. What closes it is per-axis input
curves beside the grid, which is what an ICC `A2B` tag is, and which the backend would have to be
taught. Against it stands the 48 to 51 of 255 that compositing in *somebody else's* four components
costs (ADR 0251), which is what the round removed.

## The four components were two rasters, and that is closed

**§11.3.4 applies the compositing formula per component** — "[t]he i th component of the result
colour 𝐶𝑟 shall be obtained by applying the compositing formula to the i th components of the
constituent colours" — so a rasteriser with three channels composites four by drawing the page
twice with a different three loaded. `Compositing::Subtractive(Half)` is which three; both halves
carry §11.3.4's additive complements, so the blend functions see what that clause requires without
anything being complemented around them; `pdf_render::BlendingSpace` carries the conversion out as
the ink cube's sixteen corners and `blending::resolve` applies it where §11.4.7 does, before the
medium. **`render-quorra` draws it since the four-hundred-and-thirty-ninth** and `render-gpu` still refuses
the list. `QUORRA_FEEDBACK.md` §17 asked whether two `Target::Readback` renders against one quorra
device were possible; they always were, they share their uploaded resources and cost the second
pass no geometry at all, and `89d7dd77` added the test that keeps it so. So the work was here, and
it is one private `render` used twice with `pdf_render::blending::resolve` between it and the
medium — the construction `render-cpu` already had. The three corpus pages
(`personwithdog.pdf`, `issue12798_page1_reduced.pdf`, `bug1365930.pdf`) agree with the CPU oracle
at 0.0288, 0.0093 and 0.0760 mean, and the 3.5% of the web §17 measured is what it is worth on real
files. `render-gpu`'s refusal has a test of its own now
(`headless_gpu.rs::the_gpu_refuses_a_four_component_page`), which it did not before. ADR 0275.

**ADR 0251's "second raster format" is therefore withdrawn as a requirement.** It was a true
statement about arithmetic — the ink cube is affine on no face of the cube, 48 of 255 at worst —
attached to a wrong statement about what carrying four components costs.

## The one component was one channel, and what is left of it is a choice between two routes

**§11.3.4's one-component row was never a raster question either**, which the
eight-hundred-and-sixty-fifth found by reading the same sentence ADR 0262 read for four: the
formula is per component, so a space of one component composites one number per pixel and three
equal channels are that number three times. §11.3.5.3 says it of the non-separable modes in so
many words — "[b]lending in gray colour spaces ( DeviceGray , CalGray and ICCBased gray) shall be
done by conversion to RGB, blending in RGB, and then converting back to gray" — and each of its
four functions returns a grey for two greys. `Compositing::Grey` converts every colour on the way
in and nothing converts out; a `/DeviceGray` page group is one interpretation under it and an
isolated `/DeviceGray` group on a device page one run of its content. ADR 0790.

**What is left of the row is one thing, and it is not a construction.**

- ~~**`CalGray` and `ICCBased` 'GRAY'.**~~ **Drawn since the eight-hundred-and-seventy-first**
  (ADR 0792). Their component reaches the device through §8.6.5.2's gamma or a profile's curve,
  so the space's own component is not the channel's and compositing in device grey is a
  different picture; what was built is exactly what this bullet priced — compositing in the
  space's component under `Compositing::Calibrated` and applying the curve per pixel at the end,
  `pdf_render::GreyCurve` and `resolve_grey`, the one-dimensional form of the four-component
  `BlendingSpace` and `resolve` — for the page and for an isolated group. The conversion *in*,
  which no clause states, is the curve's inverse on the greys (`colour::GreyRoute`), ADR 0263's
  right inverse one dimension down. `press_census` over all 145 archives names the crawl's
  population: three `ICCBased` 'GRAY' page groups (`1407449.pdf`, `2760152.pdf`,
  `6942624.pdf`, all `kTRC`), three `/DeviceGray` ones, and no `CalGray`; the three draw and
  agree with the references by eye. `mupdf` and `ghostscript` composite the fixture's half-alpha
  black over white in a `/Gamma 1` `CalGray` group to 188 and 187 of 255 against this tree's
  188, so the construction is the one the two references that honour the space use.
- **Which conversion *into* the grey.** This tree takes §10.4.2.2 and §10.4.2.3 — the route
  every `/Luminosity` mask has taken since ADR 0217 — and `mupdf` and `ghostscript` take
  §10.4.2.1's other one, sRGB's linear-light luminance re-encoded, which puts a pure red at 129
  of 255 where the classic weights put it at 77; `poppler` ignores the space. §10.4.2.1 makes
  §10.3 a *should* for an ICC-enabled processor and the classic algorithms a *may* for a
  less-capable one. **This bullet said the two were one decision because "a mask and a blending
  space are one sentence of §11.6.6", and the nine-hundred-and-fourth found a second sentence**
  (ADR 0850). §11.5.3's device branch converts the composited colour to `DeviceGray` "with no
  compensation for gamma or other colour calibration", and §10.3's route *is* a colour
  calibration — so for a `/Luminosity` mask group in a device space the clause states the
  classic weights outright, `mupdf` and `ghostscript` depart from it there, and this tree is
  right for a reason rather than by permission. **The open half is the other one**: §11.6.6's
  conversion into a `DeviceGray` or `CalGray` blending space for a page or an isolated group,
  where no such sentence applies. Even there the ranking is weaker than it looks — §10.3's own
  subject is "CIE-Based colour to device colour" and this conversion has a device source,
  §10.3.2 conditions its remapping on device spaces that "do not match that of the raster output
  device" while this processor's `DeviceRGB` is that device's sRGB, and the one grey round trip
  the standard writes out, §11.3.5.3's for the non-separable modes, is §10.4.2.2's pair, under
  which grey to RGB and back is the identity. So the decision is one half the size it was, it
  is still this tree's, and it is still not taken. The two would become two functions where
  `InkScale::grey_of` is one, which is now the *correct* shape rather than a cost.

## The way into a profile's press is the profile's, and the ICC `B2A` row is closed

**The section above this one said a `B2A` was not needed, and the eight-hundred-and-seventy-seventh
read the clause that says it is** (ADR 0796). §8.6.5.5 requires a blending-space profile to carry
"from CIE" information "because the group colour space shall be used as both the destination for
objects being painted within the group and the source for the group's results" — the second half
is the `A2B` this tree already sampled out, and the first is the conversion *in*, which ADR 0272
answered with a right inverse of that sampling so that an opaque mark came back exactly. §10.3.1
hands a CIE-to-CIE conversion to the ICC specification and §10.4.2.1 ranks that route above
§10.4.2.4's classic one for an ICC-enabled processor, so the profile's own table is the standard's
conversion in and the search was this tree's stand-in. `icc::Profile::to_device` reads `B2A1` over
`B2A0` in `mft1`, `mft2` and the v4 `mBA ` (matrix and M curves modelled), `Press` holds the
profile where it is bi-directional, and a colour goes in as an XYZ — its own for a CIE-based
source, sRGB's for a device one, with the compensation the way out applied undone. What an opaque
mark loses on the round trip is now the profile's own residue, which is the producer's picture
rather than one tuned to come back. A profile without the table keeps the right inverse, and the
crawl holds none: `press_census` over all 145 archives finds every one of its 287 profile presses
(187 page groups, 94 output intents, 6 `/DefaultCMYK`) carrying a `B2A` this tree evaluates, while
`doc/pdf.js` names no profile press at all.

**A one-component CIE-based mask group takes §11.5.3's own `Y` since the same round**: `CalGray`
or an `ICCBased` 'GRAY' `/CS` on a `/Luminosity` group is painted in its component under
`Compositing::Calibrated`, `/BC` is that component, and `soft_mask::luminance_derivation` composes
"convert to the CIE 1931 XYZ space and use the Y component as the luminosity" into the mask's
table — exact, on the construction ADR 0792 built for a page. `bug1721218_reduced.pdf`'s eight
such groups are the corpus witness (at most 5 of 255 on 666 pixels at two pixels per unit; the
masks are nearly binary).

~~**What is left of §11.3.4 and §11.5.3 together is three components**~~ — **drawn since the
eight-hundred-and-seventy-ninth** (ADR 0797), in both places: a `CalRGB` or `ICCBased` 'RGB '
page, isolated group or mask group composites the space's own components under
`Compositing::Additive`, and the conversion out is `pdf_render::ColourCube` — not the sampled
3 → 3 grid this paragraph priced, because the device's transfer function is fifteen levels of 255
steep at black on a linear space, but one curve per component, a grid of linear light and the
device's own curve, which is §8.6.5.3 and a matrix profile *exactly* from eight corners. A mask
takes §11.5.3's `Y` as three summed curves (`pdf_render::Luminance`). `raster_digest` over
`doc/pdf.js` moves nine pages: `transparency_group.pdf` toward `mupdf` and `ghostscript`, and
**`issue21346.pdf` away from every reference including Acrobat** — the clause's `Y` of an sRGB
group's encoded 0.25 is 0.05, and nine renderers take the device branch's 0.25. The ADR carries
the argument and the cost, and it is the owner's to rank.

~~**What §11.3.4 keeps is the route-into-grey choice below and nothing else. What §11.5.3 keeps is
two shapes**~~ — **the nine-hundred-and-fourth took one of the two and measured the other, and
the measurement is the finding** (ADRs 0850, 0851).

- **A three-component *table* profile as a mask group's `/CS` is drawn.** §11.5.3 branches on
  the *kind* of the space and not on the shape of its arithmetic, so a profile with no
  decomposition into three curves takes the same colorimetric branch: its `Y` is sampled at 33
  points an axis over the profile's own `A2B` — the samples `ColourCube` already takes for the
  conversion out of the same profile — and `pdf_render::Luminance` interpolates it trilinearly.
  The route is asked of `Presses::rgb_route`, because 36 000 profile evaluations once per
  soft-mask dictionary is what `6081357.pdf`'s 912 masks would have cost. **Population: zero**,
  in `doc/pdf.js`'s 964 documents and in the 65 720 of `CC-MAIN-2021-31` that
  `luminosity_mask_census` opens. It was drawn anyway because what stood in its place was not a
  documented choice but a *silence*: the device branch's weights on a CIE-based space, with no
  report.
- ~~**A four-component profile as a mask group's `/CS` is reported**~~ — **drawn since the
  nine-hundred-and-seventh, as §11.4.7's pair of rasters inside the mask** (ADRs 0856, 0857).
  The population two decisions had recorded as empty is **3417 groups in 181 crawl documents**,
  measured in the nine-hundred-and-fourth; the five pieces this bullet listed were built exactly
  as listed, and the list was right about all five. `pdf_render::SoftMask::black` is the second
  command list with its own backdrop, `Luminance` grew a four-axis grid, `render-cpu` draws a
  second buffer and `render-gpu` a second scene, and the confined protocol tags the pair.
  **What the reading added to the list is one word — *unconditional***: `Command::Group`'s pair
  is skipped where nothing inside the group composites, because a group's four components are
  converted to the device at the end and an opaque `Normal` mark carries its colour through
  whatever space it was carried in; a mask's four are converted to **one number** that is a
  function of all four however opaque the marks are, so the second run is taken whenever the
  space has four components. The `Y` itself is `Press::luminance`, `PRESS_SIDE⁴` samples of the
  profile's own `A2B` at the very points `sample_press` samples for the conversion out, built on
  first use because almost every press a document names is a page's and carries no mask.

## What used to block the population, and what it turned out to be

This section carried §11.7.2's second sentence as the standing blocker for one session:

> If the colour space of a graphics object within the group is not equivalent to the group's
> blending colour space, then it shall be converted to the group's colour space , and all blending
> and compositing computations shall be done in that space

and recorded that §11.7.5.3 "names §10.4.2.4 as that conversion". **It does not.** The bullets that
name the black-generation and undercolour-removal functions are §10.4.2's side of §10.4.2.1's fork;
the paragraph above them chooses a *target* and leaves the algorithm to whichever branch the
processor is on. Reading that paragraph is the whole of the four-hundred-and-twenty-seventh session,
and what it licensed was the third of the three routes this file listed — a right inverse of the
press, with gamut mapping where no preimage exists. The two measurements this section recorded as
"where the next round will be tempted" were both offers to take a shortcut without a clause, and
neither was taken: exempting `DeviceGray` would have put black text at `#231F20`, and taking
§10.4.2.4 as written would have moved every `DeviceCMYK` pixel on every page.

## How the population was found: the blending space was the wrong four documents

**This file said "4 documents, all `/DeviceCMYK`" for eighteen sessions and three of the four were
reported for the wrong reason or for no reason at all.** §11.6.6 gives a group's `/CS` effect "[f]or
isolated groups" and then hands every other case to the parent — "[f]or non-isolated groups, or if
no group colour space is specified, the group colour space shall be inherited from the parent group
or page" — and §11.4.7 puts the *page group* under that inheritance: its `/CS` "shall serve as the
default blending colour space for each page", and "[a]ll page-level compositing shall be done in the
default blending colour space of the page".

So `issue14200.pdf` was reported for a `/DeviceCMYK` on a group that states no `/I`, on a page that
states no `/Group` at all — nothing on it composites anywhere but the device's components, and the
report has gone. And five documents were departing in silence, because nothing in this tree read
§11.4.7's entry: `bug1365930`, `bug1703683_page2_reduced`, `issue12798_page1_reduced`, `issue13520`
and `personwithdog` all state a page group of `/DeviceCMYK`, so **every mark on those pages
composites in ink**. Four of them reported it then; `bug1365930` did not, because nothing on its
first page composites and the space cannot change a pixel there. **All five are drawn in ink now**,
the last three of them in the four-hundred-and-fortieth.

`crates/pdf-model/examples/group_space_census.rs` is what says this, and the thing that made it say
anything is printing the *effective* space beside the declared one. 115 of the 974 documents state a
page group `/CS`; 7 of those name a space that is not the device's three components; 71 group
dictionaries declare `/DeviceCMYK` and 96 groups actually composite in it.

## How it was priced in the four-hundred-and-fifteenth, and what survived that pricing

**A second raster format was thought to be genuinely required, and ADR 0217 gave the wrong reason
for it; the requirement itself was withdrawn in the four-hundred-and-twenty-sixth, below.** The
reason was "a painted group's result is three components"; the number of components has nothing to
do with it. §11.3.3 under `Normal` is a weighted average — §11.3.6: "the compositing formula
collapses to a simple weighted average of the backdrop and source colours" — and a convex
combination passes through an **affine** map unchanged. So the only question is whether the
conversion out of the blending space is affine over the colours the group composites.

Measured per component over 200 000–300 000 random pairs (ADR 0251):

| the conversion | worst gap between the two orders of operation |
|---|---|
| §10.4.2.5's classic `1 − min(1, c + k)`, no channel over one unit of ink | **3.3 × 10⁻¹⁶** |
| the same, with the clamp reached | 117 of 255 |
| the same, with the clamp **deferred** onto three unclamped components | **3.4 × 10⁻¹⁶** |
| **this tree's multilinear interpolation of the ink cube** (ADRs 0009, 0042) | **48 of 255** |

Under the standard's own classic formula the collapse is exact, and the clamp is deferrable by
ADR 0220's trick one clause over. Under the conversion this project chose it does not collapse at
all, because multilinear interpolation carries products of the four inks. Half of registration black
over paper is `[76.0, 66.1, 63.9]` in `DeviceCMYK` against `[127.5, 127.5, 127.5]` on the device —
**51.5 of 255**, and `compositing_in_cmyk_is_not_compositing_in_the_device_and_this_is_the_gap` pins
it and its control.

**ADR 0251 concluded from this that a four-component raster was owed**, and the arithmetic above
is right while that conclusion is not: §11.3.4's per-component formula makes four components two
rasters, which the four-hundred-and-twenty-sixth built. What the arithmetic still decides is that
compositing in ink is a *different picture* and worth having — 51.5 of 255 at the fixture, +0.100
of 255 over the whole of `personwithdog.pdf` — and that §10.4.2.5's classic conversion is not the
way to get it, because it is 115 of 255 out at the cube's corners.

## The non-isolated group, and why it fell

**A second accumulator the clause divides out again.** §11.4.4's NOTE 4 advises keeping Table
140's group alpha apart from the composite alpha, because NOTE 3's backdrop removal divides by
the first; this file, ADR 0234 and the ledger all concluded that one premultiplied raster
therefore cannot do it. The quantity the removal divides out is **multiplied straight back in**
when §11.3.3 composites the group's result onto the same backdrop, so with the Normal blend
function at the `Do` the pair collapses — exactly, for every backdrop alpha and every blend mode
inside the group — to

```text
result = (1 − w) × backdrop + w × (elements composited onto the backdrop)
```

with `w` the group's constant alpha times its soft mask. `w = 1` is NOTE 5's flattening. So the
display list gained one flag (`Command::Group`'s `isolated`), `render-cpu` seeds the group's
buffer from the surface and writes that line in one pass, and two corpus documents lost their
only report. ADR 0237 has the derivation, the 200 000-case check against the clause's own
formulas, and the three fixtures.

### What that left behind

1. ~~**A non-isolated knockout group whose elements blend *and* whose rule can show**~~ — the same
   sentence one clause over. §11.4.6 composites each element with the group's *initial* backdrop,
   which for a non-isolated knockout group is the page, so the two stages are not the pair
   `Command::Shaped` states. **Two of the three corpus witnesses this used to name were not this
   item**, which the four-hundred-and-seventy-second found by reading the clause against them
   (ADR 0307): `knockout_blend_multiply.pdf` is one element, which has nothing to knock out, so
   §11.4.6's initial backdrop *is* §11.4.4's immediate one and the group takes ADR 0237's
   construction; and `knockout_inner_backdrop.pdf` is a non-isolated group inside an **isolated**
   knockout group, which NOTE 6 gives that group's transparent initial backdrop — so it is
   §11.4.5's isolated group by definition and was being drawn correctly while reporting otherwise.
   **`issue18032.pdf` closed in the four-hundred-and-ninety-second** (ADR 0327), by exactly the
   construction the arithmetic here priced: the display list states the pair of flags, every
   element arrives as a `Command::Shaped`, and `render-cpu` keeps the initial backdrop beside the
   accumulation with a scratch per element — `f × E = S − (1 − f) × B` recovering stage a)'s
   shape-1.0 composite from an ordinary draw. One backend rather than three: the other two refuse
   by name and the frame goes to the oracle, which is the deliberate cost recorded there.
2. **A blend mode at the `Do`**, where the collapse genuinely fails and NOTE 4's second
   accumulator would genuinely be needed — 0.601 of full scale wrong if it is assumed anyway. No
   corpus document states one.
3. **`render-gpu` refuses the command**, because a Vello layer begins transparent and cannot be
   seeded from the surface; the frame goes to the CPU backend, which is what `CLAUDE.md` keeps
   that backend for. **`render-quorra` draws it since the four-hundred-and-thirty-eighth** (ADR
   0274): `quorra_scene::GroupSpec` gained Table 145's `/I` at `89d7dd77`, which is exactly what
   `doc/QUORRA_FEEDBACK.md` §16 asked for, and the flag passes straight through. Three of the four
   corpus pages that had moved from `agree` to `refused` went back to `agree` — this time about
   the picture the clause states rather than about the one both backends were substituting — and
   the fourth turned out to be §11.4.7's, which is the row above.

## The knockout shape, and why it fell

**A shape is a second quantity a *command* can state, where the other two are second quantities a
*buffer* has to hold.** §11.6.4.2 gives an object's shape from its geometry alone; §11.6.4.3's soft
mask and §11.6.4.4's constant are opacity. So `pdf_render::Command::Shaped` carries the object
beside a second command — the object with those two removed — whose drawn alpha *is* the shape, and
a group's shape is the union of its elements'. §11.4.6's two stages then come to
`P' = (1 − f) × P + S` in premultiplied form, which both backends draw as Destination-Out with the
shape and then **Plus** with the object. ADR 0234, and its third fixture is the one that pins the
Plus: source-over there is 32 of 255 out at a half-covered pixel under a half-opaque mark.

### What that left behind, each reported by name and each with no corpus witness

1. **An element whose one alpha carries both quantities in a raster.** An image's samples may be
   §8.9.6.2's stencil (shape) or §11.6.5.2's `/SMask` (opacity), and a shading's colours already
   carry §11.6.4.4's constant, so neither can be un-multiplied after the fact. An `ImageSource`
   that keeps the two apart would answer both, and it is a smaller construction than the
   population below.
2. ~~**§11.6.4.3's `/AIS`.**~~ — **closed in the five-hundred-and-eightieth, ADR 0415**, and the
   price this entry quoted was an overstatement of a construction that turned out to be an
   identity. It said honouring the flag "means composing the mask and the constants into the shape
   instead of into the object, which is a second `stated_shape` rather than a new vocabulary" —
   right about the shape of the answer, and composing them *into* the shape yields the element
   back. §11.6.4.2 makes an elementary object's intrinsic opacity 1.0 everywhere and the flag hands
   the mask and both constants to shape, so §11.3.7.2's source opacity is 1.0 and §11.3.7.1's alpha
   is the source shape: the shape command is the element with its blend mode dropped. **What this
   entry got wrong besides the price was the population**: it said nine corpus documents state the
   entry and "none of their knockout groups is drawn today", which had stopped being true when ADR
   0327 scoped the flag — measured over every page of all nine, not one of their knockout groups
   reaches the refusal. The world population was **1 of 65 944** crawled web documents. What is
   refused in its place is a *scope*: a group whose content painted under **both** readings, and,
   under the shape reading, a non-isolated group used as an element — whose accumulated alpha
   carries its backdrop's beside its own, which is item 1's debt one level up.
3. ~~**`render-quorra` refuses a `Shaped` element outright**~~ — **closed in the
   four-hundred-and-fifty-sixth, ADR 0291.** The history is the part worth keeping, because it is
   three rounds long and each one was a different kind of wrong. §14 asked for Destination-Out and
   Plus and **both arrived at `89d7dd77`** (quorra's ADR 0025), weighted by shape rather than by
   the paint's alpha, which is what a `Shaped` command's second member already carries; the
   four-hundred-and-thirty-ninth session then wrote the translation out and found neither mark
   could be *asked for*, for two independent reasons — `SceneBuilder::fill` refused a staged
   operator inside a knockout group, which is the one position `Command::Shaped` occurs in, and
   `group`, `stroke` and `image` carried no `Compose` at all while three of the four corpus pages
   state a `Shaped` whose halves are **groups** (§11.6.4.2 makes a nested group's shape the union
   of its elements'; only `knockout_smask.pdf` is a fill-and-fill pair). `doc/QUORRA_FEEDBACK.md`
   §14.2 asked for both lifts; quorra's ADRs 0032 and 0033 are both, at `2c9bdd0`.

   **What this side wrote is one rule rather than two constructions**: a half that is already a
   group states the operator on its own `GroupSpec`, and every other half is drawn inside a group
   of one element — the same arithmetic, and the only uniform route in a vocabulary that carries
   `Compose` on a fill and on a group and on nothing else. A stroke, an image and a fill whose
   paint is a sampled shading all take the second route, and a per-mark route would have been
   correct for some paints and silently wrong for the rest. **It is still written as a pair or not
   at all** — `Plus` alone saturates a premultiplied channel past its alpha, and the library states
   that as the caller's obligation because one mark cannot tell it whether the other is coming.
   All four corpus pages agree with the CPU oracle, and `quorra_states_what_it_will_not_stage` now
   holds the two constraints that *did* survive — a staged half may carry no blend mode (§11.3.5's
   implicit one-element group) and must be isolated (§11.4.4's backdrop would arrive inside the
   shape).
4. **`render-gpu`'s coverage path keeps its documented residue**: where the shape *is* the coverage
   it still draws the element with source-over after the Destination-Out, which weights the
   backdrop by `1 − f × opacity` a second time. Bounded and stated in `knock_out`'s own comment
   since the seventy-first session. Removing it means a Plus layer per element and the elements are
   §9.3.8's glyphs, so it wants a measurement before it is paid.

## What the five precedents have in common, and what the sixth was instead

`ImageSource` carries a raster the list *names* (ADR 0210), a mask group is painted in the one
quantity §11.5.3 composites (ADR 0220), a knockout element states its shape beside its colour
(ADR 0234), a group names the backdrop its elements composite onto (ADR 0237), and a page's four
components are a second *list* beside the first (ADR 0262). In each the missing quantity turned out
to be sayable in a command — three times as a second command or a second raster, twice as a flag or
a field over an identity nobody had derived.

**The sixth was not a quantity at all**, and that is why this paragraph used to end by saying it was
unsayable: a conversion *into* the blending space is a function rather than a value, and no display
list can carry one. What it needed was not a command but a **branch decision** — §11.7.5.3 names the
conversion's target and §10.4.2.1 ranks the algorithms, so the conversion in belongs on whichever
branch the conversion out is on, and a right inverse of the ink cube is what that branch means here
(ADR 0263). **The seventh is the fifth's shape one scope down and closed the same way** (ADR 0327):
a group's own four components are a second command list beside the first with the conversion out as
a value — `pdf_render::GroupBlending` — and §11.4.6's other backdrop is a *discipline* over buffers
a backend already has rather than a new quantity, the initial backdrop retained beside the
accumulation. What the residues left in the table still need is a conversion **between two
presses**, per pixel, at a group boundary — a function of a function, which really is not a
quantity a command could name, and which no corpus document asks for.
