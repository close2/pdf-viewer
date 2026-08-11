# ADR 0277 — The black component's rule was not a second blend function

Date: 2026-08-11 (session 441)
Status: accepted

## Context

After ADR 0276 took §11.6.6's row from 85 web documents to 8, the largest transparency
population `doc/todo/23` had was **§11.3.5.3's non-separable blend at 31 of 65 944** — a page
that states §11.4.7's `/Group << /S /Transparency /CS /DeviceCMYK >>` and paints anything under
`Hue`, `Saturation`, `Color` or `Luminosity`. Such a page was drawn on the device's three
components and reported by name, on a reason the ledger and this tree had carried since ADR
0262:

> a non-separable blend mode gives the black component a rule of its own (§11.3.5.3), and
> neither raster has a blend function that states it

`issue18032.pdf` is the one corpus witness.

## What the clause says, and where it splits

§11.3.5.3 states its four modes in full — `Lum`, `ClipColor`, `SetLum` and `SetSat` as
pseudocode — and it says what they operate on:

> These functions operate on colours that are assumed to have red, green, and blue components.

Three, which is what each of ADR 0262's two rasters holds. The CMYK rule is two bullets:

> The C , M and Y components shall be converted to their complementary R , G and B components
> by subtracting each from 1.0. The formulae in this subclause shall be applied to the RGB
> colour values. The results shall be complemented back to C , M and Y in the same way.

> For the K component, the result shall be the K component of Cb for the Hue , Saturation , and
> Color blend modes; it shall be the K component of Cs for the Luminosity blend mode.

The first bullet is the **chromatic raster's own contents**. §11.3.4 requires a subtractive
space's components to be "complemented (subtracted from 1.0) before the blend function is
applied", and `Half::Chromatic` stores that complement, so a backend's `Hue` already sees the R,
G and B this clause names — nothing is mapped and nothing is complemented around it.

The second bullet is what the report was about, and it is the one that reads like a second blend
function.

## The arithmetic, which is that it is not one

The black raster is **neutral in all three of its channels** by construction: `Half::Black`
paints `Color::grey(1 − black)`, so every colour, every image sample, every shading stop and
every anti-aliased edge on it has `r = g = b`. On a neutral pair each of the clause's own
auxiliary functions degenerates:

- `Sat(C)` is `Cmax − Cmin`, which is **0**;
- `SetSat(C, 0)` reaches the clause's own `else` arm — `if Cmax > Cmin` is false for a neutral
  operand — and returns black;
- `SetLum(C, l)` on a neutral `C` adds one offset to three equal components, so it returns the
  neutral colour of luminosity `l`, and `ClipColor` has nothing to clip because `l` came from a
  colour already in range;
- `Lum` of a neutral colour is that colour's own level, because the clause's three weights sum
  to one.

So Table 135's four come to

| mode | `B(Cb, Cs)` on a neutral pair |
|---|---|
| `Hue` | `SetLum(SetSat(Cs, 0), Lum(Cb))` = `SetLum(black, Lum(Cb))` = **`Cb`** |
| `Saturation` | `SetLum(SetSat(Cb, 0), Lum(Cb))` = **`Cb`** |
| `Color` | `SetLum(Cs, Lum(Cb))` = **`Cb`** |
| `Luminosity` | `SetLum(Cb, Lum(Cs))` = **`Cs`** |

which is the second bullet, term for term. **The rule the clause states for the black component
is a theorem about the formulas it states for the other three**, evaluated at the colour that
component is carried as.

Checked over 200 000 neutral pairs — 500 backdrop levels against 400 source levels, so that the
weights and `SetSat`'s test are met at every magnitude rather than at round numbers — the worst
gap is **1.19 × 10⁻⁷**, one ulp at 1.0, and it is a residue with a name: `0.3 + 0.59 + 0.11` is
not exactly 1 in binary floating point, so `Lum` of a neutral colour is its level times
`1 ± 6 × 10⁻⁸`. Three hundred-thousandths of one level of 255.
`the_clauses_own_functions_give_the_black_components_rule_on_a_neutral_pair` is that check.

## What was built

**Nothing, and that is the finding.** The change is the deletion of the refusal in
`Interpreter::blending_undrawable`, which is now three conditions rather than four — and all
three of the survivors want a *second colour space*, which is what that function's own comment
said the four had in common while one of them did not.

The route not taken is worth recording, because it was written first and then withdrawn. A
`BlendMode::Backdrop` member for `B(Cb, Cs) = Cb`, emitted onto the black list by a
`BlendMode::black_component` mapping applied in `DisplayList::set_blending`, would have said the
clause's second bullet explicitly. It has an exact collapse of its own — substituting `B = Cb`
into §11.3.3 gives `αr × Cr = αb × Cb + (1 − αb) × αs × Cs`, which in premultiplied form is
Porter-Duff **Destination-Over** — and `tiny_skia::BlendMode::DestinationOver` and
`peniko::Compose::DestOver` both have it. Three things decided against it:

1. **`quorra_scene::Compose` does not**, at `89d7dd77`: it offers `SrcOver`, `Src`, `DestOut`
   and `Plus`, and no pair of those composes to Destination-Over either, because the weight it
   puts on the source is the *destination's* alpha and no source-side operator supplies one. So
   the explicit route costs the quorra backend all 31 documents and gains no picture the
   identity above does not already give.
2. It adds a member to `pdf_render::BlendMode` that no `/BM` produces and that every backend has
   to carry, for a rule the backends already implement.
3. The identity is not an approximation to be trusted — it is checkable, and it is checked in
   two places: against the clause's arithmetic in `render-cpu`'s own unit test, and **across
   backends** by the fixture below, which is what trap 2 asks for.

`render-gpu` is unaffected either way: it refuses a list whose `blending()` is `Some` before it
reaches a blend mode at all.

## The fixtures, and what each is held to

Three in `pdf-model/tests/transparency_groups.rs`, over a page that states
`/Group << /S /Transparency /CS /DeviceCMYK >>` with two **opaque** page-covering fills, so
§11.3.3 runs at `αb = αs = 1` and reduces to `Cr = B(Cb, Cs)` — the pixel *is* the blend
function. The clause's `Lum`, `ClipColor`, `SetLum`, `SetSat` and Table 135 are transcribed in
the test file, so the expectation comes from §11.3.5.3 rather than from `render-cpu`'s
transcription of it.

- `a_non_separable_blend_takes_the_black_component_from_the_backdrop`. `Cb` = `1 0 0 0.4 k`
  under `Cs` = `0 1 0 0 k` in `Hue`. Complementing gives (0, 1, 1) and (1, 0, 1); `Sat(Cb)` is 1
  so `SetSat` is the identity; `Lum(Cb)` is 0.70 against `Lum(Cs)`'s 0.41, so `SetLum` adds 0.29
  and reaches (1.29, 0.29, 1.29) — out of range, so `ClipColor` is load-bearing and scales about
  `L` = 0.70 by 0.30/0.59 to (1.0, 0.4915, 1.0). The ink is `0 0.5085 0` and **the K is the
  backdrop's 0.4**, which no part of that arithmetic produced. Through the assumed press:
  **(161.4, 81.3, 124.2)**. Taking the source's K instead is (245.3, 125.3, 196.5).
- `the_luminosity_mode_takes_the_black_component_from_the_source`, the same two colours one
  bullet down: `SetLum(Cb, Lum(Cs))` subtracts 0.29, `ClipColor`'s *first* arm scales about
  `L` = 0.41 by 0.41/0.70, the ink is `1 0.4143 0.4143` and the K is **0**.
- `every_non_separable_mode_agrees_with_the_clause_over_four_components` uses `0.2 0.6 0.9 0.4`
  under `0.7 0.1 0.3 0` for a reason: the pair above makes both saturations 1, so `Hue` and
  `Color` are the same picture there. These four are four pictures, which is what the last loop
  asserts — a construction that folded the four into one would satisfy everything else.

And a **fifth mark on the shared cross-backend scene**, `test_scenes::four_component_page`: the
same `Hue` pair, opaque, whose value is **(12, 88, 90)** against (19, 138, 141) for the wrong K.
`render-cpu`'s `four_component_page.rs` derives it and `render-quorra`'s
`cpu_and_quorra_agree_on_a_four_component_page` asserts it on the graphics device, which is where
the identity stops being a claim about arithmetic and becomes a claim about two independent
implementations. It passes. (The scene's paper probe moved from page y 250 to y 420, the band
between the two rows of marks, because the new pair took the first.)

**Putting the old route back**, three ways, each failing exactly what it should:

| what was put back | which tests fail |
|---|---|
| the refusal in `blending_undrawable` | all three fixtures — the page falls to the device |
| the K rule replaced by the source's (`Hue` → `Normal` on the black raster) | the `Hue` fixture and the four-mode one; `Luminosity` passes, correctly |
| Destination-Over replaced by source-over on the black raster | the same two |

## What the gates said

Every gate in `doc/todo/02` §2 was run **on the stashed tree and again after**, in this round.

| | before | after |
|---|---|---|
| tests | 1614 | **1614** (+1 doctest = 1615) |
| corpus, 974 documents | 65 incomplete | **65** |
| oracle verdicts | 905 / 68 / 786 | **identical, all seven counts** |
| oracle complete / incomplete | 1693 / 101 | **identical** |
| quorra | 917 / 35 / 5 / 17 | **identical, name for name** |
| text vs `pdftotext` | 99.2% (24007/24191), 62 not gated | identical |
| text vs PDFBox | 99.8% (14257/14281) | identical |
| dates, XMP, JPEG 2000 | 1514/1545, 318/1, 14 | identical |
| citations / quotations | 6505 / 618 | **6529 / 624** |
| ledger | 875 rows, 406/248/18/82/8/113 | identical |

The corpus count does not move because `issue18032.pdf` keeps two other reports — §11.4.4's
non-isolated group and §11.4.6's knockout — and loses only its third. **quorra being identical
name for name is the sharpest line here**: it draws §11.4.7's two rasters (ADR 0275), the black
raster of `issue18032.pdf` now carries a `Hue`, and the backend agrees with the oracle about it.

**`doc/todo/00`'s step 7, over all 786 ambiguous pages, before and after: every line is
byte-identical**, numbers and labels both. Twenty names sit at or past −1 and they are the same
twenty. The alarm holds for the sixteenth consecutive run. `issue18032.pdf` is not in that
bucket, so the picture was checked directly instead.

## The picture, which is where the change is

`issue18032.pdf` page 1 at 2×, before and after: **RMSE 22.86 of 65535**, page ink 1.28942 →
1.29393. The difference image is empty everywhere except **inside the one gradient rectangle the
page draws**, where it is a set of horizontal bands — a `Hue` over a shading, composited in ink
rather than in light. Nothing else on the page moved, which is what a change confined to one
clause looks like.

## The web population, before and after

65 944 documents, 145 archives, one process apiece, both passes with no failure of any kind:

| | before | after |
|---|---|---|
| **incomplete** | 851 | **824** |
| blending reports, all conditions | 83 over 82 documents | **52 over 51** |
| page-level reports (§11.4.7) | 53 | **22** |
| a non-separable blend mode (§11.3.5.3) | **31** | **0** |
| a group inside the page composites in a different space (§11.6.6) | 8 | 8 |
| an `/ExtGState` states Table 57's `/BG` or `/UCR` (§11.7.5.3) | 9 | 9 |
| four components that are not four this tree can sample | 5 | 5 |
| a group *introduces* a space on a page that states none | 30 | 30 |
| §11.4.4's non-isolated group | 135 over 129 | identical |
| §11.4.6's knockout | 23 over 20 | identical |

**27 of the 31 become complete and 4 keep a report they already had**, all four §11.4.4's
non-isolated group — `0100121.pdf`, `0792036.pdf`, `1776874.pdf` and `7065802.pdf`. **Nothing
joined the incomplete set**, and the two rows below the closed one are identical document for
document, which is the check that the 27 left because their last condition stopped firing rather
than because a condition was narrowed (trap 5).

That is 27 documents of the web made complete for a change that wrote no code, and it is the
largest such move in this chain after ADR 0276's 54. **The largest transparency row the web has is
now §11.4.4's non-isolated group at 129**, which is a different file's row; the largest inside
`doc/todo/23` is the 30 that introduce a space.

## Consequences

- **`doc/todo/23`'s §11.3.5.3 row is closed**, with no code written for it and no vocabulary
  added. What is left of that file is one construction (a group on the page in a space of its
  own, 8 + 30 web documents), §11.7.5.3's black generation (9), five unsampleable page groups,
  and the two backend rows §11.4.6 still owns.
- **The clause's own algebra collapsed the construction for the fifth time in this chain** —
  ADRs 0220, 0234, 0237, 0262 and now this — and this one collapsed it *further* than the round
  set out to: the first draft was a display-list member and a mapping, and the arithmetic said
  neither was owed.
- **A neutral raster is now load-bearing and is documented as such.** `Half::Black` paints
  `Color::grey`, and the reason that matters is no longer only "so a backend may read any
  channel": it is what makes §11.3.5.3's black-component rule fall out of the same shader every
  other page uses. A future change that let the black raster carry anything else in its other
  two channels would break this silently, which is why the identity has a test rather than a
  paragraph.
- **The ledger row that named this a blocker was wrong for fifteen rounds**, in the shape
  `doc/ledger-and-claims.md` lists first: a row describing what the code *should* do. It was
  written in the four-hundred-and-twenty-sixth session with the two rasters, and nobody asked
  what the four functions return on the raster it was about.
