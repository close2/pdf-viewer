# 1254 — What the standard says one profile is worth, and what a cube of 33 costs

Status: accepted. Session 1208.
Context: ISO 32000-2 §11.3.4, §11.4.7, §11.5.3, §11.6.6, §11.7.2, §11.7.5.3, §8.6.5.9, Table 69,
Annex C; trap 38; ADRs 0416, 0417, 0790, 0850, 1056, 1207, 1230, 1242, 1243.
Code: `crates/pdf-colour/src/colour.rs`, `crates/pdf-model/src/content/report.rs`,
`crates/pdf-model/examples/press_depth.rs`.
Documents: §11.4.7, §11.5.3, §11.6.6, §11.7.2 and §11.3.4's ledger rows, `doc/todo/23`,
`doc/todo/65`.

## 1. `colour::MAX_PRESSES` was a round number, and trap 38 asks what a bound owes the standard

Four rows ended on the same sentence: a page that has already named `MAX_PRESSES` distinct
presses has none left for the next four-component space it states. The constant was 8, chosen in
session 582 when the budget moved from the process to the interpretation (ADR 0417), and nothing
above it said where 8 came from.

**The standard states a number, and it is about one profile rather than about a page.**
§11.7.5.3's second bullet makes the conversion out of a group a function of the graphics state at
the `Do` — the rendering intent in effect there — and `PressIdentity::Profile` is keyed on exactly
that. Two entries of the state select it: Table 69 names four rendering intents, and §8.6.5.9
makes `/UseBlackPtComp` a second parameter of the same conversion while pinning one of the eight
pairs, since an absolute intent forces compensation off. Four intents times two settings less
that pair is **seven**. So a page embedding one profile and painting it under every intent the
standard defines names seven presses, and any bound below seven refuses a conformant page that
named a single profile.

Nothing in the standard bounds how many *profiles* a page may name. **Annex C is informative and
states no number for this at all** — its architectural table covers integers, arrays,
dictionaries, nested constructs, names, strings and CIDs, and its one colour entry is a
recommendation about `DeviceN` colourants in previous versions.

## 2. What documents actually do

`pdf-model --example press_depth` is new. It interprets pages and reads
`Interpretation::presses_named`, which is the number the budget is compared against — a question
about a *page*, which `examples/press_census`'s count of the distinct presses a whole population
names could not answer.

| population | documents | pages | deepest page |
|---|---|---|---|
| `doc/pdf.js`, whole | 963 opened of 974 | 1 283 | **0 presses** |
| the crawl, every eleventh document, three pages each | 5 977 opened of 5 995 | 13 188 | **1 press** |

A page compositing in `/DeviceCMYK` with no profile behind it counts none of this, by design:
`assumed_press` is a pure function of a compile-time constant and no budget applies to a press no
file asked for.

## 3. The bound

`MAX_PRESSES = RENDERINGS_OF_ONE_PROFILE * 2`, fourteen, with both halves stated above the
constant: the first is the clause's floor, the second is margin over everything measured. An
interpretation that spent the whole of it would hold about 15 MB of press, against 8.6 MB before.
The refusal stays a report — `PagePress::Beyond` — and is never a panic.

`MAX_CACHED_PRESSES` is deliberately **not** tied to it and stays at eight: one is what a page
may name and the other what a process keeps warm, and ADR 0417's whole point is that a cache may
be sized against a population where a budget may not.

**§11.6.6 and §11.7.2 take `departed`**, their one departure being this bound with its cost above
the constant. §11.4.7 and §11.5.3 stay `partial`, each on a second requirement of its own: a
reference XObject's imported page composited under the containing page's group attributes rather
than its own (§8.10.4, nothing on this disk states one), and a blend mode inside a subtractive
group of more than one component.

## 4. The cube into a parent's space is not affine, and now has a number

§11.3.4's row and `doc/todo/23` carried a question about the "non-affine route" into a blending
space. Two answers, and the first is the clause's.

**`Lab` is not part of the question.** §11.3.4 says the Lab space and ICCBased spaces separating
lightness from chromaticity "shall not be used as blending colour spaces because the compositing
computations in such spaces do not give meaningful results when applied separately to each
component". A space the clause forbids is the document's failure, not a route this tree owes.

**The cube is a sampled lookup, so nothing here is non-representable.**
`pdf_render::ColourCube` is per-axis input curves, a `side³` grid and an output curve;
`transparency::into_parent_cube` samples `parent_channels` over device RGB at
`INTO_PARENT_SIDE` = 33 with **identity** input curves. So the whole conversion, non-affine
stages included, sits in the grid — and `ColourCube`'s own doc comment says why that is the wrong
place for a steep stage.

Measured, trilinear interpolation of that grid against evaluating `RgbRoute::components_of_srgb`
directly, worst of 200 000 uniformly random colours, in levels of 255:

| side | `CalRGB` `/Gamma 2.2` | `CalRGB` `/Gamma 1` |
|---|---|---|
| 2 | 8.55 | 73.25 |
| 9 | 7.54 | 1.47 |
| 17 | 6.24 | 0.37 |
| 33 (what is built) | **4.66** | 0.09 |
| 65 | 3.40 | 0.04 |

The linear space converges as a smooth function should and is already exact for practical
purposes; the gamma-2.2 space does not converge at all usefully, because the conversion's last
stage is an inverse gamma whose derivative is unbounded at zero and a uniform grid cannot
resolve it. Raising the side is not the fix — quadrupling it bought 1.26 levels.

**The fix is named and not taken here**: give the cube the parent's own per-component curve as
its input curves, which is exactly what those three stages exist for and what
`own_space_conversion` already does for the conversion *out*, where two samples an axis reproduce
§8.6.5.3's stages exactly. That is a change to what gets drawn on the 31 documents of 88 890 that
reach this path, so it is a round with `raster_golden` and the oracle behind it rather than a
paragraph in this one. `doc/todo/23` carries it and §11.3.4's row names the number.
