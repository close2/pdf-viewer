# 0452 — The wash a gradient asked for around itself

Status: accepted.
Session: 616. Corrects ADR 0151's correction of ADR 0150, which was right about the tree and had
nothing to say about how long it would stay right.

## What the clause requires

ISO 32000-2 §8.7.4.3, Table 77, and it is a `shall`:

> ( Optional ) An array of colour components appropriate to the colour space, specifying a single
> background colour value. If present, this colour shall be used, before any painting operation
> involving the shading, to fill those portions of the area to be painted that lie outside the
> bounds of the shading object.

with, three sentences later in the same cell, the condition that decides where the requirement
lives at all:

> The background colour shall be applied only when the shading is used as part of a shading
> pattern, not when painted directly with the sh operator.

§8.7.4.5.2 states the same entry again for a type 1 shading, in the form that names the geometry:

> Points wi thin the shading's bounding box ( BBox ) that fall outside this transformed domain
> rectangle shall be painted with the shading's background colour ( Background ); if the shading
> dictionary has no Background entry, such points shall be left unpainted.

## What this tree did

Nothing, and said nothing. `pdf_model::shading` never read the key; no `pdf_render::Shading` has a
field for it; the interpreter's `domain_clip` — the one function whose doc comment quoted
§8.7.4.5.2's sentence in full — closed with "a shading that states one gets the same treatment
silently".

That sentence is the finding. **The gap was known and written down in three places**, and each of
the three described it accurately: the ledger's §8.7.4.3 row, the comment in `pattern.rs`, and ADR
0151's own closing section, which exists because ADR 0150 had claimed the entry was "unimplemented
and reported" and it was not. **A gap that three documents agree about is not a gap anybody is
going to act on.** What none of the three did was make the *program* say it — so a reader looking
at `issue13372.pdf` saw a page with a hole in it and a report list that did not mention the hole.

## The decision

**Read the entry where the clause makes it mean something, report it, and do not paint it yet.**

`pdf_model::shading::background_components` returns how many components Table 77's `/Background`
states, and `Interpreter::pattern` raises `Unsupported::ShadingBackground` from the
`/PatternType 2` branch and from nowhere else. `sh` owes nothing and reports nothing, which is the
clause's own exemption and not a convenience: a report raised on `sh` would fire on every page in
the corpus that paints a shading directly, and a report that fires where nothing is missing is
what stops anybody reading the list.

The component count rather than a boolean, because the array's *length* is what says whether the
file's entry is usable — Table 77 requires "an array of colour components appropriate to the
colour space", so a caller that knows the space knows whether this one is one.

## Why the paint is not in this round, and it is a reading rather than a budget

The construction that suggests itself is Table 77's own NOTE 1:

> In the opaque imaging model, the effect is as if the painting operation were performed twice:
> first with the background colour and then with the shading.

Two operations, one solid fill and one shading, over the same path. It is four lines in the
interpreter and it is wrong on this device, twice, and the NOTE says why in its own first clause —
*in the opaque imaging model*.

- **At the path's boundary.** This rasteriser anti-aliases, which is §10.7.4's departure (1), so a
  boundary pixel carries a fractional coverage `c`. Two marks at coverage `c` leave `(1 − c)²` of
  the backdrop where one leaves `1 − c`. The background colour would therefore appear as a fringe
  around every path such a pattern fills — the maximal case of the conflation ADR 0308 measured on
  a *seam*, because here the two marks are not merely abutting but coincident.
- **Under §11.6.4.4's alpha.** `ca` is a property of the painting operation. Performing it twice
  applies it twice: inside the shading's bounds the result is `ca·S + (1 − ca)·(ca·B + (1 − ca)·D)`
  against the clause's `ca·S + (1 − ca)·D`.

So the exact construction is the other one — **the shading's paint answers the background colour
where it would otherwise answer nothing** — which is one operation, one coverage and one alpha, and
which produces exactly what the normative sentence describes: the area to be painted covered by the
background outside the bounds and by the shading inside them.

That is a field on `pdf_render::Shading`, resolved through the shading's own colour space, honoured
by three backends across four shading kinds. Three of the twelve cells are a few lines each
(`stops()`'s transparent stop becomes an opaque background stop; `MeshRaster` and `RadialRaster`
clear to the background instead of to transparency), one is already there
(`quorra_scene::Paint::Function` has a `background` field nothing sets), and one is **not this
tree's**: quorra's gradient lane carries `extend` and no background, so an axial or radial shading
with a background either goes upstream, or takes the raster lane the radial cone case already uses,
or leaves one backend disagreeing with another — which is the thing the cross-backend gate exists
to refuse. `doc/todo/17` holds the table and the three answers.

**Reporting first is not a way of avoiding the paint.** It is trap 5's rule, and it is what turns a
defect three documents agreed about into one a run prints.

## The population, and how it was taken

`witness_census` over every PDF on this disk — 1251, of which 1239 open — finds **five** documents
stating the name `/Background` as a name. Reading all five: two state Table 77's, and three state
an optional content group called `Background` or a `/PieceInfo` `/Private /Background`, which is a
different key with the same spelling. So the population is **2 of the 974 and 2 of the 1249**, and
both use the shading as a `/PatternType 2` pattern, which is the only case the clause's condition
admits.

Both draw wrong today. `issue13372.pdf`'s axial shading states `/Coords [90 108 522 684]` and no
`/Extend`, so the default `[false false]` bounds it to the band between the perpendiculars through
those two points; its pattern fills a CCITT stencil over the whole 595 × 842 page, whose corners
project outside `[0, 1]` on that axis. `issue18816.pdf` states one on a `/ShadingType 6` Coons
patch mesh, where the bounds are the union of the patches.

## What else the same reading corrected

- **§8.9.6.4's count was stale.** "Both corpus instances" is three — `colorkeymask.pdf`,
  `issue14821.pdf`, `issue15629.pdf` — and all three still reach the unpacker unfiltered, which is
  what the sentence was actually about. The row also never named its population; it does now, and
  the four `doc/corpora/` submodules add nothing.
- **§8.7.4.1's negative claim holds**, re-derived rather than re-read (`doc/todo/01`'s sixteenth
  sweep, ADR 0405): 38 of the 974 hold a `/PatternType 2` object, none of them states an
  `/ExtGState`, and the submodules hold no Type 2 pattern at all.
- **§8.6.6.5 gained an erratum in this reader's favour.** Errata Collection 3 Issue #309 strikes
  "which may be present only for DeviceN colour spaces that do not have the NChannel subtype" out
  of the sentence that gives `None` its *meaning*, and states the restriction separately as a
  `shall not` on the file. `ColourSpace::parse_at` reads no `/Subtype` and applies the rule to
  every `DeviceN`; the base text left that arguable and the amended text ratifies it.
