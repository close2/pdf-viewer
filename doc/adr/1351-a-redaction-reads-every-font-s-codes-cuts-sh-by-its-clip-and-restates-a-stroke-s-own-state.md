# 1351 — A redaction reads every font's codes, cuts `sh` by its clip, and restates a stroke's own state

Status: accepted and **built**.
Context: `crates/pdf-transform/src/redact.rs` (`Codes`, `Type3Marks`, `Walk::code_box`,
`split_codes`, `Walk::restate_stroke`, `GraphicsState::non_stroking_colour`, `with_states`),
`crates/pdf-transform/src/redact/shading.rs`, `crates/pdf-font/src/composite.rs`
(`composite_cmap`, now public), `crates/pdf-transform/tests/redact.rs`.
Supersedes: ADR 1124's refusals of a Type 3 font, a composite font not `Identity-H` and the `sh`
operator; ADR 1236's refusals of a stroke with no stroking colour operator and of `/CA` differing
from `/ca`.

§12.5.6.23 asks a processor to "remove all traces of the specified content", and these refusals
stood because the walk did not know where a code's marks fall, how long a code is, or how to paint
the survivors of a stroke in the stroke's own state. Each is a reading of the clause that defines
the content. Two premises of the brief were wrong and are corrected here: Type 3 fonts are §9.6.4
(the tree cited §9.6.5, character encoding), and `sh` is §8.7.4.2 (not §8.7.4.5.2, which is
function-based shadings).

## 1. A Type 3 glyph is tested against the box its description declares

A Type 3 font is a simple font — one byte, one code — whose glyph is a content stream, so its marks
can lie outside the advance box the interpreter places. §9.6.4 states where they lie: Table 111's
`d1` operands are the glyph's bounding box, and "[t]he declared bounding box shall be correct - in
other words, sufficiently large to enclose the entire glyph", and for a `d0` glyph Table 110's `/FontBBox` is
"the smallest rectangle enclosing all marks that would result if all of the glyphs of the font were
placed with their origins coincident". That box goes through `/FontMatrix` and §9.4.4's text
rendering matrix into the display list's space, and the code's box is its union with the placed
quadrilateral. A code whose box meets the region is removed whole — the bounding-box choice ADR 1124
made for every glyph — including a glyph whose description straddles the region edge.

The code is removed from the string and its advance restored as a `TJ` adjustment exactly as for any
other font: §9.4.3 says the number "shall be expressed in thousandths of a unit of text space",
whatever `/FontMatrix` says about glyph space, and the adjustment is read back from the placed
quadrilaterals, which already carry `/FontMatrix`. The glyph description is the font's, drawn
wherever else the code is shown, and it stays.

Refused: a `d0` glyph in a font whose `/FontBBox` is all zero. Table 110 then says "a PDF processor
shall make no assumptions about glyph sizes based on the font bounding box", and nothing else states
where the marks fall.

## 2. A composite code takes its own bytes, as the codespace ranges delimit them

§9.7.6.2: "The codespace ranges in the CMap … specify how many bytes are extracted from the string
for each successive character code." The walk resolves the `CMap` with the font loader's own
function (`pdf_font::composite_cmap`, made public for this), so a predefined name, an embedded
stream and its `/UseCMap` chain are read one way, and it splits with `CMap::next_code`, the
function the reader decodes with, which covers §9.7.6.3's rule for a code outside every range too.
A removed code takes exactly its bytes; the kept ones are written back byte for byte. The
code-count calibration (trap 13) still holds the walk to the interpreter. The odd-length refusal
for `Identity-H` goes with it, since §9.7.6.3 says how many bytes a trailing half code consumes.

Refused: a `CMap` that does not resolve, and a vertical one. In writing mode 1 §9.4.4's
displacement is `ty`, and the gap this walk restores is along the text space's x axis.

## 3. `sh` is cut by its clip, where the shading holds no data of its own in the region

Table 76's `sh` paints "subject to the current clipping path", so its marks are the clip and
nothing in the stream names them one by one. The walk tracks a box that contains the clip. A
shading whose clip box and `/BBox` miss the region is left alone. One that meets it is painted
through the clip intersected with the region's complement: the painted window, mapped back through
the inverse transform, is cut by ADR 1195's nine-cell construction and ADR 1236's margin proof, and
the pieces are set as a clip inside a balanced `q … Q`.

That is a clip hiding marks, and the clause forbids that for image data: "clipping or image masks
shall not be used to hide that data". So it is taken only where the shading holds nothing of its own
located in the region. An axial or radial shading whose function is §7.10.3's exponential, or
§7.10.4's stitching of exponentials, states a colour at each end of each piece and nothing about
where any of it lands. A function-based shading, a mesh, and a sampled or calculator function place
data in the plane. Cutting the clip would hide that data rather than destroy it, so the page is
refused.

The same reading reaches a path painted in a §8.7.4 **shading pattern**. The path is cut, and the
pattern goes on being named by the survivors, so it is carried whole. A pattern whose shading holds
located data is therefore refused on the same terms. A §8.7.3 tiling pattern's cell repeats across
the plane and holds nothing that belongs to the region alone.

## 4. A stroke's survivors are filled in the stroke's own colour, alpha and overprint

- **Colour.** ADR 1236 refused a stroke with no stroking colour operator on the ground that a walk
  which had entered a form could not assert §8.6.8's initial value. That no longer holds. Every
  stream the walk runs is entered from a state it tracked: a page starts at Table 51's initial
  values, a form starts from the state at its `Do`, and a mask group starts from the initial state
  (ADR 1352). So such a stroke is black in `DeviceGray`, stated as `0 g`. An `SC` or `SCN` with no
  `CS` before it is replayed after `/DeviceGray cs`, because the non-stroking space need not still
  be the initial one.
- **Alpha and overprint.** Table 51 keeps two alpha constants and two overprint parameters, and no
  operator sets any of them. Where a stroke's values differ from the fill's, the survivors are
  filled under a new graphics state dictionary. It holds the stroke's `/CA` as `/ca` and its
  overprint as `/op`, and the stream's resources gain it under an unused name (`with_states`).
  It restates the producer's own values and invents nothing. A transparency group form resets both
  alphas on entry, which Table 51 requires.
- **Seams.** Every surviving piece is one `f`, so the pieces are one mark and nothing composites
  twice where they meet. The dash pattern is applied before the cut (ADR 1236).
- **Refused:** a zero line width, which §8.4.3.2 states in device pixels.
- **A documented choice:** a stroke under `/SA true`. §10.7.5 adjusts it in device space, so it has
  no user-space outline. The survivors are the unadjusted outline, which differs from the adjusted
  stroke by at most the half pixel that clause allows it. The marks inside the region are gone
  either way.
