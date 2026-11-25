# Specification audit of ISO 32000-2 §8.1 to §8.3, §8.8 and §10.1 to §10.4 (graphics overview, coordinate systems, external objects; rendering, CIE-based and device colour conversions) — round 2237

The project's sixteenth specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2234 ([`2234-specification-audit-7-7-7-12.md`](2234-specification-audit-7-7-7-12.md)).
Baseline: main at `b0f478b8`. Evidence:
[`2237-specification-audit-8-1-8-3-8-8-10-1-10-4/`](2237-specification-audit-8-1-8-3-8-8-10-1-10-4/)
— `findings.json` (2038's schema) and `expected-failures.json` (the failing tests of each
behaviour finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing** but one documentation slip (D01). A fix round takes the
findings by the groups at the end.

## Method

1. The `shall`s and `should`s of §8.1 to §8.3 (`doc/md/ISO_32000-2_sponsored_EC3.md` lines 4112 to
   4345), §8.8 (6238 to 6257, Table 86) and §10.1 to §10.4 (8715 to 8858) were read and set against
   the code that performs the step. Most of §8.1 to §8.3 is definition: the processor-side
   sentences are §8.3.2.3's default user space (`/CropBox`, `/Rotate`, `/UserUnit`), §8.3.2.4's
   five spaces and the matrix each is mapped by, §8.3.4's premultiplication, and §8.2's ordering
   of the transparency stack. §8.7.2's pattern space and §8.10.4's reference proxies were read
   where the brief named them.
2. 2038's findings on these clauses were read first and are not repeated (T08 on negative UCR,
   T32 on non-commuting `cm`, T33 on mid-path `q`, D38 on the classic-conversion notes, D62 on a
   singular `FontMatrix`), nor 2184's B01 and D01 on `/BG` and `/UCR` in a `d1` glyph. Errata:
   `doc/errata/site/docs/32000-2-2020/clause08.md` H8.2 (Table 50's categories) and `clause10.md`
   H10.3.1 (the ICC reference becomes Table 66's versions) and H10.4.2.2 and H10.4.2.3 (`grey`
   respelled `gray` in the formulas); Issue #128 (H7.8.3) for a glyph's resources. None settles a
   finding below.
3. The code traced: `pdf-model`'s `content.rs` (`base_transform`, `displayed_size`, the `base` and
   `pattern_initial` fields), `content/run.rs` and `run/graphics.rs` (`cm`), `content/xobject.rs`
   (`Do`, the form and proxy matrices, the imported page), `content/text/type3.rs`,
   `content/pattern.rs` and `pattern/tiling.rs`, `page.rs` (`/UserUnit`), `type3.rs`
   (`/FontMatrix`), `output_intent.rs` (`grey_reading`); `pdf-colour`'s `black_generation.rs`,
   `colour/to_cmyk.rs`, `intent_grey.rs`, `press.rs`, `spots.rs`, `device_cmyk.rs` and
   `ink_search.rs` by their doc comments.
4. The lens was *every route to a thing* applied to two constructions: the space a pattern matrix
   maps to, which the interpreter keeps in one field swapped at each kind of nested content stream
   (§7.8.2 lists five), and `DeviceGray` on a CMYK press, which this tree carries as a
   `Separation` named `DeviceGray` — a name, so *a rule keyed on a name applies wherever the name
   is read*, including the spot planes.
5. Each B finding has a failing `*_spec_audit` test whose control passes on main:
   `pdf-model/tests/coordinates_8_3_10_4_spec_audit.rs`, a module of `content_suite`.

What holds, probed by reading, is not listed one by one. Among it: `cm` premultiplies
(`matrix.then(state.transform)`), and a form's `/Matrix` and a proxy's do the same; default user
space shifted to the crop box, rotated clockwise by `/Rotate` and scaled by `/UserUnit` last, with
`/UserUnit` honoured in `displayed_size` and every inverse a viewer takes; a `/UserUnit` that is
not a positive number read as 1 and logged; a Type 3 glyph run under `FontMatrix × text space`
with the rest of the state inherited; images in the unit square; `Do` on a `/Subtype` other than
`/Image` and `/Form` (a PDF 1.x `/PS` among them, which PDF 2.0 no longer defines) reported and
not drawn; a pattern placed against a form's space at its `Do`, an annotation appearance's, a
tiling cell's, a soft mask group's and an imported page's; §10.4.2.4's complete conversion with the
file's `/BG2`/`/UCR2` precedence, `k` as the operand of both and the clamps on the results;
§10.4.2.3's black `1 − gray` under a CMYK intent; `DeviceCMYK` to the screen by §10.3.2's licence
rather than §10.4.2.5's formula, as §10.4.2.1 ranks them.

Readings left, each a question rather than a defect found: a `/Matrix` of other than six numbers is
read three ways — a form drawn by `Do` takes the first six numbers and logs it, the same form drawn
as an appearance stream takes the identity, a pattern's and a shading's take the identity — which is
the silent-repair campaign's question (ADR 3477), not a clause's. A Type 3 font stating no
`/Resources` draws from the page's (Issue #128's last step); whether a pattern found there is
placed against the page's space or the glyph's is left to B01's fix round.

## Findings

Three behaviour gaps. In order of how much a person would see:

- **2237-B01** — a pattern a Type 3 glyph description paints with, from the font's or the glyph's
  own `/Resources`, is placed against the page's default space (or the enclosing form's), not the
  glyph description's. `Interpreter::base` is swapped at every other nested stream; `run` resets
  its companion `pattern_initial` for a glyph and `base` is left as it was. A glyph filled with a
  100-point-wide shading draws one twentieth of its ramp.
- **2237-B02** — on the simulated press under a CMYK intent, a page that names a spot colourant
  `/DeviceGray` gets every `DeviceGray` colour painted on that spot's plate, because the grey's
  construction is a `Separation` of that name and `SpotSet::available` compares names (§10.2:
  device colours "shall be rendered as process colours").
- **2237-B03** — the simulated press of a page with no CMYK intent is the assumed inks, a CMYK
  device, but `grey_reading` keys on the intent alone, so `DeviceGray` is separated into four inks
  as a neutral rather than black `1 − gray` (§10.3.2's `should`, §10.4.2.3).

Two documentation findings: D01 (fixed), `device_ink_press`'s documentation cited §10.4.2.2 for a
grey that enters as black alone, which is §10.4.2.3; D02, the `base` field's documentation and the
comment over a shading pattern's placement say "the page's default space" where §8.7.2 says the
parent content stream's, left for B01's fix, which rewrites those lines.

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `8.3.2.4`, `8.7.2` and
`9.6.4` (B01); `10.2` (B02); `10.3.2`, `10.4.2.3` and `10.8.3` (B02, B03).

## Suggested fix groups

1. **`pdf-model/src/content/text/type3.rs`'s `draw_type3_glyph`** — B01 and D02: swap
   `Interpreter::base` for the glyph's initial transform around its run, as `xobject.rs`,
   `annotations.rs`, `pattern/cell.rs` and `transparency/soft_mask.rs` do, and correct the two
   comments.
2. **`pdf-colour/src/colour/spots.rs`'s `SpotSet::named`, `reach` and the overprint questions, or
   `intent_grey.rs`'s construction** — B02: the grey's construction names no plane, whatever the
   page names.
3. **`pdf-model/src/output_intent.rs`'s `grey_reading` and `content.rs`'s `simulated_press`** — B03:
   with the simulation asked for and no four-component intent, the assumed press's black ink alone
   (ADR 2712's construction over `assumed_inks_profile`); the screen view unchanged. Groups 2 and 3
   touch one construction and may be one round.
