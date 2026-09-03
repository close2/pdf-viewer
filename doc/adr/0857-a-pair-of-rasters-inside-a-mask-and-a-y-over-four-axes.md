# ADR 0857 — A pair of rasters inside a mask, a `Y` over four axes, and 41 crawl pages that move away from every reference

Status: accepted. Session 907.
Clauses: ISO 32000-2 §11.5.3, §11.3.4, §11.4.7, §11.6.5.1, §11.6.6, §11.7.2, §8.6.5.1, §8.6.5.5,
§8.6.5.9, §10.4.2.3.
Code: `crates/pdf-render/src/soft_mask.rs` (`BlackHalf`, `SoftMask::black`,
`SoftMask::paired_value`, `paired_values`, `Luminance::ink_grid` and `multilinear`),
`crates/pdf-model/src/colour.rs` (`Press::luminance`),
`crates/pdf-model/src/soft_mask.rs` (`luminosity`, the four-component branch, the report's new
words), `crates/pdf-model/src/content/transparency.rs` (`Interpreter::mask_halves`),
`crates/render-cpu/src/lib.rs` (`build_soft_mask`'s second buffer, `mask_values`),
`crates/render-gpu/src/soft_mask.rs` (the second scene),
`crates/render-quorra/src/scene.rs` (the refusal's words),
`crates/pdf-render/src/repeat.rs` and `src/group_cost.rs` (the second list),
`crates/viewer-confined/src/protocol/display_list.rs` (the pair on the wire),
`crates/pdf-model/examples/press_census.rs` (`--luminance`),
`crates/pdf-model/examples/luminosity_mask_census.rs` (the shape's wording).
Tests: `crates/pdf-render/src/soft_mask.rs::a_luminance_over_four_axes_reads_the_pairs_own_components`,
`crates/pdf-model/tests/transparency_groups.rs::a_four_component_mask_group_takes_the_y_of_its_pair_of_rasters`,
`crates/pdf-model/tests/transparency_groups.rs::a_mask_group_in_a_cie_space_with_no_route_is_named_rather_than_drawn_in_silence`,
`crates/viewer-confined/src/protocol/display_list.rs::a_four_component_masks_pair_round_trips_to_an_equal_list`.
Continues ADRs 0262, 0272, 0327, 0796, 0797, 0851. Beside ADR 0856, which is the reading.

## What was built

ADR 0856 states what §11.5.3 requires of a mask group whose blending colour space has four
components and why the pair of rasters is the only exact carrier. This is the construction, and
it is the five pieces `doc/todo/23` listed, with one word added by the reading.

- **`pdf_render::SoftMask::black`** — a `BlackHalf` of a second command list and a second
  backdrop. The mask group's content stream is interpreted twice: `SoftMask::commands` under
  `Compositing::Subtractive(Half::Chromatic, press)` and this under `Half::Black`, with the
  half of §11.6.5.1's four-component `/BC` each raster composites onto.
- **`Interpreter::mask_halves`** takes the second run, with `readback_mark`/`rewind_readback`
  around it so the second interpretation's text and glyph counts do not arrive twice, and
  `paired` afterwards so two runs that drew different structures are not read against each
  other. **Unconditional where the space has four components**, which is the word ADR 0856's
  reading added to the list: `group_commands` skips its own second run where nothing inside the
  group composites, and that argument does not survive the move into a mask, because a mask's
  four components become one number that is a function of all four however opaque the marks are.
  A `0 0 0 1 k` fill paints nothing at all into the chromatic raster.
- **`pdf_render::Luminance` grew a fourth axis.** `Luminance::ink_grid`, `as_ink_grid` and
  `axes()` beside the curves and the three-axis grid, with `trilinear` generalised to
  `multilinear` over `2^axes` corners — the same weights and the same index order as
  `BlendingSpace::convert`, cyan fastest and black slowest.
- **`Press::luminance`** samples it: `PRESS_SIDE⁴` values of `profile.to_xyz_with(inks, false)[1]`
  at the very grid points `sample_press` already samples for the conversion out. Without
  §8.6.5.9's black point compensation, as `RgbRoute::luminance` is one axis down — the clause
  asks for the colour's XYZ and the compensation is a step toward a destination.
- **The backends.** `render-cpu` draws the black half into a second buffer over the same rows
  and reads the pair through `SoftMask::paired_value`; `render-gpu` builds a second scene and
  reads it back the same way, calling the oracle's own function on it; `render-quorra` refuses
  by name — `quorra_scene::MaskKind::Luminosity` weighs the channels of one body in its own
  shader — and `doc/QUORRA_FEEDBACK.md` §43 now asks for a second body beside the curves-or-grid
  field it already asked for. The confined protocol tags the four-axis grid separately from the
  three-axis one and carries the black half's list and backdrop.

## What it cost

**The fourth dimension is cheap and it is *more* faithful than the conversion out beside it.**
`examples/press_census --luminance` was written for this and run over all 145 archives of
`CC-MAIN-2021-31`, which name **287 profile presses**; for each it builds the grid and compares
interpolating it against evaluating the profile over 20 000 ink quadruples:

| | median | p90 | worst |
|---|---|---|---|
| the sampled `Y`, in levels of 255 | **0.17** | **0.98** | **0.98** |
| the sampled *device colour* at the same side (ADR 0272) | 5.99 | 11.02 | 14.52 |

Under one level of 255 everywhere, against six of them for the colour on the identical grid — and
the reason is the one ADR 0272 wrote down for its own residue: that grid is sampled uniformly in
ink and read out in sRGB, whose transfer function is fifteen levels steep at black, where this one
is a smooth scalar in linear light. So no argument about the side was needed here; `PRESS_SIDE` is
already finer than the quantity being sampled.

**The time is 11.8 ms in the median and 58.9 ms at worst**, once per press per process, over 203
cold samplings in that run. It is behind a `OnceLock` on the `Press` rather than inside
`sample_press` for `CLAUDE.md`'s launch rule: 187 of the crawl's presses are a *page* group's and
94 an output intent's, and none of those carries a mask, so paying it in `sample_press` would put
83 521 profile evaluations on the launch path of every four-component page for a number no such
page reads. Memory is 334 KB a press, a third of the grid beside it.

## What it drew, and the thing this round has to say plainly

**41 of the 181 crawl documents that state such a group move their first page**, and **38 of the
41 move away from the reference consensus.** Every one of the 181 was rendered at scale 1.0 with
the branch on and with it off, in one sitting, and the three references were run over the 41 that
moved:

| | before | after | |
|---|---|---|---|
| mean \|ours − poppler\|, 40 pages | 18.728 | 19.502 | **+0.774** |
| mean \|ours − mupdf\|, 41 pages | 17.195 | 18.038 | **+0.843** |
| mean \|ours − ghostscript\|, 17 pages | 14.367 | 15.837 | **+1.470** |

**This is ADR 0797's disagreement one component count over, and it is the same sentence of the
same clause.** That ADR took `CalRGB` and three-component profiles down §11.5.3's colorimetric
branch and recorded that `issue21346.pdf` then moved away from nine renderers including Acrobat,
because the clause's `Y` of an encoded mid-grey is its *linear* luminance — 0.05 for an encoded
0.25 — where every reference takes the device branch's number. Four components do the same thing
for the same reason: a mask fading through an encoded mid-grey fades much faster under the `Y`
than under EXAMPLE 2's weights. `4605565.pdf` p1 is the pinned witness and shows both directions
at once — its "Soup of the Day" panel moves *to* poppler's, mupdf's and ghostscript's bright
yellow, and its salad panel's gradient fades further from all three — for one arithmetic.

Principle 5 is what decides it: agreement is evidence about our reading and disagreement is a
question for the specification, and the question was already taken there. §11.5.3 branches on the
space's kind, §11.3.4 lists 'CMYK' beside 'GRAY' and 'RGB ', §8.6.5.1 makes all three CIE-based,
and the device branch's own sentence — "with no compensation for gamma or other colour
calibration" — is written for spaces that have none. Refusing four components while drawing one
and three would not be caution; it would be an accepted decision applied to two of its three
cases. **So it is drawn, and the cost is stated here rather than discovered later: the ranking is
the owner's, and it is now the same ranking for one, three and four components rather than three
separate ones.**

**No page of `doc/pdf.js` changes**, which is what the census said in advance — the corpus states
no four-component mask group — and what the raster gates then confirm.

## Two things worth keeping from how this was measured

- **A stale example binary nearly produced a false negative.** The first before/after pass
  reported *zero* of 181 pages moved, and the cause was that `cargo build --example X` had been
  run for other examples in between, so `render_at` on disk was still the binary from the
  branch-off build — trap 10b's shape, in a `--release` example rather than in a module. What
  caught it was running the pass a third time against a *planted* defect and finding the "after"
  and "before" panels differing from it by exactly the same pixel counts.
- **A null result has to be run against the defect before it is believed** (trap 13), and doing
  that twice is what made the number trustworthy: inverting the sampled `Y` moves 41 pages by up
  to 252 of 255, and a bulge of at most 16 of 255 confined to the *middle* of the `Y` — zero at
  both ends — moves the same 41 by up to 101. The second is what rules out "the masks are binary,
  so both routes agree", which was the only innocent explanation for the first pass's zero.
