# 879 — Three components composited, and a cube out: `CalRGB` and `ICCBased` 'RGB ' blending spaces composite in their own components, a three-component mask group takes §11.5.3's `Y` as three curves, and on an sRGB mask the clause parts from every reference

Date: 2026-09-03.
ADR: [0797](../adr/0797-three-components-composited-and-a-cube-out-and-the-clauses-y-parts-from-every-reference.md).
Touched: `crates/pdf-render/src/blending.rs`, `crates/pdf-render/src/display_list.rs`,
`crates/pdf-render/src/soft_mask.rs`, `crates/pdf-render/src/lib.rs`,
`crates/pdf-render/src/group_cost.rs`, `crates/pdf-render/src/repeat.rs`,
`crates/pdf-model/src/colour.rs`, `crates/pdf-model/src/icc.rs`, `crates/pdf-model/src/soft_mask.rs`,
`crates/pdf-model/src/image.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/content/transparency.rs`, `crates/pdf-model/src/content/image.rs`,
`crates/pdf-model/src/content/pattern.rs`, `crates/pdf-model/examples/luminosity_mask_census.rs`,
`crates/pdf-model/tests/transparency_groups.rs`, `crates/pdf-model/tests/oracle.rs`,
`crates/render-cpu/src/lib.rs` and four of its tests, `crates/render-quorra/src/lib.rs`,
`crates/render-quorra/src/scene.rs`, `crates/render-quorra/tests/corpus.rs`,
`crates/render-quorra/tests/headless_quorra.rs`, `crates/render-gpu/src/lib.rs`,
`crates/render-gpu/src/scene.rs`, `crates/render-gpu/tests/headless_gpu.rs`,
`crates/test-scenes/src/lib.rs`, `crates/viewer-confined/src/protocol/display_list.rs`,
`doc/conformance/ledger.toml`, `doc/todo/23-transparency-departures.md`, `doc/state-of-play.md`,
`doc/verify.md`, `doc/QUORRA_FEEDBACK.md`. A worktree round on `round-879`, branched from
`main` with `round-877` merged in, not merged.

## The spec track: §11.3.4's and §11.5.3's three-component remainder

Round 877 left one shape in both rows: a `CalRGB` or `ICCBased` 'RGB ' blending space composited
on the device's three channels. §11.3.4 lists both among the spaces a processor "shall" support
and forbids `Lab` in so many words; the per-component sentence ADRs 0262, 0790 and 0792 built on
makes a three-component space three composited numbers; §8.6.5.3 and a profile's `A2B` are the
conversion out, applied where §11.4.7 puts it; and §11.7.2 decides most of the conversion in — a
device colour of three components "shall be the CIE-based space of the nearest such ancestor" for
compositing purposes, so a `DeviceRGB` mark keeps its numbers, while a CIE-based colour goes in
from its XYZ and a device colour of another count from sRGB's, as ADR 0796 chose for a press. The
same clause's sentence about a nested group is carried out too: a device-space group of the same
colourant count inside a CIE-based one *is* that space and changes nothing, which is what keeps a
`/DeviceRGB` group inside an sRGB page — the commonest group there is — off the fallback. And a
matrix profile is read as bi-directional on the clause's own definition: its matrix inverts and its
curves are monotone, so `Profile::to_device` runs its stages backwards without a `B2A`.

The construction is not round 877's sketch. A single sampled 3 → 3 grid interpolates across the
device transfer function's toe at black and is fifteen levels of 255 out on a linear space, so
`pdf_render::ColourCube` is one curve per component, a grid in linear light and the device's own
curve — exact from eight corners for `CalRGB` and a matrix profile, because everything between the
curves is linear, and a grid of 33 an axis for a table profile. The mask side is §11.5.3's `Y`
where it is a sum of one function of each component — the clause's own EXAMPLE 1 for `CalRGB`, a
matrix profile's curves weighted by the middle row of its matrix — carried as three 256-sample
curves (`pdf_render::Luminance`) the backend sums before Table 142's `/TR`; `/BC` is three numbers
in the space. `Compositing::Additive` and `colour::RgbRoute` are the interpreter's side, with a
route cache per interpretation capped at the press's eight. `render-cpu` draws all of it,
`render-quorra` applies the page cube on its readback and refuses a group cube and a three-curve
mask by name, `render-gpu` refuses both cubes, and `viewer-confined` writes a third tag for each.
Tests derive every value from the clauses on a linear `CalRGB` of sRGB's primaries and a matrix
profile of sRGB's colourants: half of black over white at `srgb_encode(½)`'s 188, a `DeviceRGB`
grey of ½ reinterpreted to 188 beside a `DeviceGray` of ½ converted in and back to 128, a pure
green's `Y` of 0.7152 against the device branch's 0.59.

## Witnesses, and the finding

`examples/raster_digest` over `doc/pdf.js` moves nine first pages, rendered before and after at two
pixels per unit beside three references. `transparency_group.pdf` moved toward `mupdf` and
`ghostscript` (1.10 → 0.18 and 1.40 → 0.45 mean levels) and away from `poppler`, which draws the
page on the device; `issue16742.pdf`, the one `CalRGB` page group, moved on 450 edge pixels and
stays within 0.12 of `mupdf`; six sRGB-profile pages moved by a level on a few pixels.

**`issue21346.pdf` moved from exact agreement with `poppler` to 18.78 mean levels from all three
references, and pdf.js issue #21346's reporter had already put Acrobat and eight renderers on the
same side.** A Typst mask group in an sRGB profile composites white at `/ca 0.25` over black to an
encoded 0.25; §11.5.3's colorimetric branch — "convert to the CIE 1931 XYZ space and use the Y
component as the luminosity" — takes it through the profile's tone curve to 0.05, and every other
reader weighs the encoded 0.25 with no gamma, which is the device branch the clause states for
device spaces only. The reading was checked against EXAMPLE 1's gamma and §11.7.2's NOTES 3 and 4
and kept, per principle 5; the ADR records that this is the first decision in this tree to part
from every reference on a population the crawl counts in the tens of thousands, and that it is the
owner's to rank. The oracle names the page in a new group, `CONTRADICTED_LUMINOSITY_OF_A_CIE_BASED_MASK`,
with the bound it fails in the gate's words; the quorra gate names it and the two group-cube pages
in `REFUSED_BEFORE_THE_SCENE`, and `doc/QUORRA_FEEDBACK.md` section 43 is the ask for both shapes.
`bug1721218_reduced.pdf` does not move. `doc/todo/00`'s step 7 sweep over the 835 ambiguous pages
puts the three movers among them within 0.9 of 255 of the lightest reference, with the twenty names
past −1 the same population the file already describes.

## Gates

The whole of `doc/todo/02` §2 in the worktree, in order, each corpus line under
`tools/bounded.sh --tree 12` and one at a time after checking for a neighbour's walk: fmt and
clippy under `-D warnings` (four findings on the first run, fixed — two functions over a hundred
lines, a `from_` name, a pair of identical match arms), nextest (3005 passed), doctests, both
`fuzz/` lines, the sandbox build, corpus (974 documents, 64 incomplete), the hayro build, oracle
(1945 pages, 835 ambiguous, 61 contradicted, 47 not comparable — red once on `issue21346.pdf` newly
contradicted and held by no group, green after the group was written), text, both censuses, dates,
xmp, jpeg2000, quorra (958 pages, 929 agree, 22 differ, 7 refused — red once on the three names
above, green after they were named), fixed documents, transform, the writer's walk and conformance
— every one green on its last run. The oracle ran against a copy of the main build directory's
reference cache (`PDFREF_CACHE`), never the cache itself. Round 878's gates ran on `main` beside
this round's builds. `--bin quoted`, `--bin unpriced` and `--bin overtaken` were run over the
oracle's log; none names this round's note.

## For the next round

- §11.5.3: a three-component *table* profile as a mask group's `/CS` (the `Y` is not separable;
  no member in `doc/pdf.js`), and a four-component profile as one (§11.4.7's pair inside a mask,
  no corpus member).
- §11.3.4: ADR 0790's route-into-grey choice, the row's last debt.
- The owner's ranking of `issue21346.pdf`'s reading — the clause against Acrobat and every
  reference on the crawl's majority mask population — and, if a convention is to be offered beside
  the clause, the host-supplied policy shape principle 3 already has.
- `doc/QUORRA_FEEDBACK.md` section 43's two asks.
