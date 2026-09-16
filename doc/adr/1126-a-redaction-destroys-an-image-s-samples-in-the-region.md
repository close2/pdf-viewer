# 1126 — A redaction destroys an image's samples within the region

Status: accepted. Session 1119.
Context: `crates/pdf-transform/src/redact.rs` (the `do_xobject`, `plan_image_clear`, `exclusively_owned`,
`clear_images`, `cleared_image_samples`, `clear_region`, `zero_sample`, `build_cleared_image`,
`reference_counts` items), `crates/pdf-transform/src/lib.rs` (`Origin::Redacted { images }`),
`crates/pdf-transform/tests/redact.rs`.
Builds: ADR 1124 (round 1112's text removal and its refusal set), `pdf_syntax::serialize::flate_encode`,
`pdf_model::image::Document::image_stream`, `pdf_model::colour::ColourSpace`.
Clauses: ISO 32000-2 §12.5.6.23 (Table 195), §8.9.5, §8.9.5.2, §8.9.6.2, §8.9.7, §8.5.

## 1. The image case ADR 1124 refused is built

§12.5.6.23: "If a portion of an image is contained in a redaction region, that portion of the
image data shall be destroyed; clipping or image masks shall not be used to hide that data." Round
1112 refused every image meeting the region. This round destroys the samples of an **image
XObject** whose placement meets the region, so the row's owed image capability is met for the case
this build can prove clean.

The samples come from `Document::image_stream`, which runs every filter *before* the codec: for a
codec-free image (`FlateDecode`, `LZWDecode`, `ASCII85` …) the bytes it returns are the packed
samples themselves. Each sample whose centre — mapped through the placing transform into the display
list's space, the same space ADR 1124 maps the region into — lies in a region box has every one of
its `components × BitsPerComponent` bits set to **zero**, MSB-first within the row as §8.9.5.2 packs
them. The image is then re-encoded `FlateDecode` and written as a **new stream** whose slot is
reserved before the closure walk (as a replaced page's is), so the original samples are copied by
nothing and no orphan holds them.

**Why zero.** §8.9.5.2 maps an integer sample of value 0 through any `/Decode` array to that array's
`Dmin` — one constant, carrying none of the original sample, whatever the colour space or an
inverted `/Decode`. The annotation's `/IC` and `/OverlayText` describe the *appearance* drawn over
the cleared region; composing them is A65's authoring departure (ADR 1124 §3), so the samples carry
no colour of their own and the constant is the sample domain's zero, not black.

**Only the region, and exactly.** The region is inverse-mapped into image space to bound the work,
then every sample centre in that block is mapped forward and tested against the region boxes exactly
— so a rotated placement clears only the samples truly inside, and the "outside is byte-identical"
half is exact to sample granularity, not an over-cleared bounding box.

## 2. A shared image is refused, not cleared — the single-referrer guard

Overwriting an object's samples changes bytes every placement of it shares. So an image is cleared
only when it is proven to belong to the redacted page alone: referenced exactly once across the
whole document (`reference_counts`), reached through a `/Resources` and `/XObject` path the page does
not share. A shared image is **refused by name** (principle 1: an image partly cleared for one page
and wrong for another is the defect this exists to prevent). Over-refusal is safe; the census (round
1119) found no corpus `/Redact` meets any image at all, so the guard costs nothing real.

## 3. What stays refused, each with a narrower reason

Read against §12.5.6.23's "content identified by the redaction annotation", three marks stay
refusals rather than silences:

- an **image behind a codec** (§8.9.5 `DCTDecode`/`JPXDecode`/`CCITTFaxDecode`/`JBIG2Decode`) or one
  whose grid this build cannot count — the samples are not re-encodable here without a codec writer;
- an **inline image** (§8.9.7) — clearing its samples is a content-stream splice this build does not
  yet do (an XObject's samples live in their own object; an inline image's are in the content stream);
- a **painted path or form** (§8.5) — removing only the portion of a vector mark within the region
  needs geometric path subtraction, and deleting the whole painting operator would destroy content
  the annotation did not identify (the mark's bbox reaches outside the region), which is the opposite
  failure from the one the clause forbids. This is the reading the round was asked to make: a vector
  mark is not removable "the way text is", because a glyph's advance box is the unit removed and a
  path has no such per-region unit without subtraction.

The row stays `partial`: these are owed capabilities, not a pure departure.
