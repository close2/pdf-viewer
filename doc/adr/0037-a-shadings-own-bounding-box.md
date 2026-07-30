# ADR 0037 — A shading's own bounding box

Status: accepted, 2026-07-30.

## Context

§8.7 — patterns and shadings — was 19 `unreviewed` ledger rows out of 20, with §8.7.3.1 and
§8.7.4.5.3 among the clauses in `REVIEW_OWED`. All seven shading types and both pattern types
have worked since early sessions, which is exactly the shape the ledger exists to find: a
clause whose obvious half is implemented and whose remaining sentences nobody has read.

The demand track pointed at the same family. `CONTRADICTED_UNEXPLAINED` in `oracle.rs` named
one identified live cause among its 60 pages:

> `mesh_shading_empty.pdf` draws the same mesh as the references, displaced horizontally.
> A placement question rather than a missing feature.

## What the measurement said

**That diagnosis is wrong.** Reading the two renders pixel by pixel: our coloured region
begins at column 0 and ends at column 199, and so does every reference's. At the vertical
centre our colours are within a level of poppler's, mupdf's and ghostscript's. The mean
distance is 0.62 against a bound of 1.00 and the worst tile 2.12 against 5.00 — **the only
failing metric is structural similarity, 0.972 against 0.990**, and what it is measuring is
the faint lattice left by filling a Gouraud triangle as many flat sub-triangles.

The one pixel where a reference differs by more than 22 levels is one where *poppler* differs
from us **and** from mupdf.

So the entry was a hypothesis wearing a diagnosis's clothes, which makes trap 1's tally five
for five. The comment now records what was measured, and what closing it would take: a Gouraud
rasteriser in **both** backends, since the cross-backend scenes hold them to identical pixels
and neither `tiny-skia` nor Vello has the primitive.

## Decision

Reading the family found what the corpus could not rank, and one of the findings fixed a
contradicted page.

### Table 77's `/BBox` is a clip, and it is now applied

> If present, this bounding box shall be applied as a temporary clipping boundary when the
> shading is painted, in addition to the current clipping path and any other clipping
> boundaries in effect at that time.

Two phrases decide the implementation. **"In addition to"** means it nests inside the state's
clip rather than replacing it, so it is one more link in the clip chain the display list
already has. **"The coordinates shall be interpreted in the shading's target coordinate
space"** means the caller's transform carries it, not the shading's own — which for `sh` is the
current user space and for a pattern is the pattern space, and which is why a Type 1 shading's
`/Matrix`, composed *inside* `shading::build`, must not be applied to it.

For a shading pattern the box cannot be turned into a clip when `scn` selects the pattern,
because the clause fixes the clip in force "at that time" — the time being the paint, possibly
several `q` levels later. So `PatternPaint::Shading` carries the rectangle and its transform,
and each paint site builds the clip against whatever the state holds then.

**Five corpus documents write a shading `/BBox`, and this changed one of them.**
`issue8092.pdf` page 1 left `CONTRADICTED` — the first movement in that count for four
sessions. It had been filed under *substituted fonts*, a list whose name has now failed to
diagnose a member six times.

The other four change by no pixel, because their boxes are at least as large as the path or
page the shading was already confined to. So the clause's general case has no corpus witness
and `shadings.rs::a_shadings_bbox_clips_what_it_paints` is the whole of what defends it — trap
8, once more.

### What the family found and this session did not implement

Each is recorded in its ledger row with a count, so that the next session chooses from
measurements rather than from impressions.

- **§8.7.4.3's `/Background`**, which fills "those portions of the area to be painted that lie
  outside the bounds of the shading object" and only under a pattern, never under `sh`. Two
  corpus documents.
- **§8.7.4.3's `/AntiAlias`**, a hint with a default of false, in the same class as §10.7.2's
  flatness. Nineteen documents.
- **§8.7.4.1's `/ExtGState`** on a Type 2 pattern — "graphics state parameters to be put into
  effect temporarily while the shading pattern is painted". **Zero** corpus documents, measured
  by walking every Type 2 pattern dictionary in all 974.
- **§8.7.3.1's `/TilingType`**, whose three values describe adjustments to the *device pixel
  grid*. This renderer distorts nothing and snaps nothing, which is value 2's behaviour
  whatever the file asks for; 21 documents ask for something else.
- **§8.7.4.4's interpolation space.** The clause says a CIE-based shading's "gradient fill
  calculations shall be performed in that space", with conversion to device colours "only after
  all interpolation calculations have been performed" — and NOTE 3 exempts shadings with a
  `/Function`, which is every Type 2 and Type 3. So the rule bites only on a mesh whose vertex
  colours are given directly, and there this tree converts each vertex when the mesh is read
  and interpolates in device RGB.

## Consequences

| | before | after |
|---|---|---|
| pages agreeing with the reference consensus | 814 | **815** |
| pages contradicted | 116 | **115** |
| of those, on pages we call complete | 102 | **101** |
| ledger subclauses unreviewed | 448 | **428** |

The unreviewed count fell by 20, the largest single-session fall so far, and the contradicted
count moved for the first time in four sessions — by a clause the corpus could not have ranked,
found by reading the family that a *wrongly diagnosed* contradicted page pointed at. Both
tracks did their job and neither would have got here alone.
