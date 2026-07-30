# ADR 0051 — One raster instead of four thousand triangles

Status: accepted, 2026-07-31.

## Context

§8.7.4.5.5 asks for Gouraud interpolation across a mesh triangle. Neither `tiny-skia` nor
Vello has a Gouraud primitive, so both backends subdivided each triangle by quarters until its
corner colours agreed to within 1/512 — or six levels, 4096 pieces — and filled each piece
flat.

That produced **three** defects, and only the first was ever written down:

1. **A lattice.** A flat piece takes the *mean* of its corners, which on a ramp is not the
   colour at any of its pixels, so the pieces meet at visible steps.
2. **A bias.** The mean is systematically wrong in the same direction across a whole gradient,
   which is why `issue2948.pdf`'s green channel read 3 to 48 where every reference reads 0.
3. **Seams needing a departure to close.** Two abutting antialiased edges do not sum to full
   coverage, so every piece had to be grown by `SEAM_OVERLAP = 0.8` pixels — a constant chosen
   by sweeping it against the cross-backend test until the two backends agreed, which is
   tuning rather than reading.

`mesh_shading_empty.pdf` was `CONTRADICTED_UNEXPLAINED`'s one *identified* live cause, and its
entry said closing it "needs a Gouraud rasteriser in **both** backends, since the cross-backend
scenes hold them to identical pixels".

## Decision

**`pdf_render::MeshRaster` rasterises the mesh once, at device resolution, and both backends
draw the result as an image.**

- A pixel's colour is §8.7.4.5.5's interpolation evaluated at that pixel's **centre**,
  barycentrically.
- A pixel belongs to a triangle when its centre does — no antialiasing, no partial coverage.
  That is what makes adjacent triangles tile *exactly*: every sample falls on one side of a
  shared edge or the other, and a sample exactly on the edge is claimed by both with the later
  one winning. **There are no seams to repair**, so `SEAM_OVERLAP` is gone, and so are
  `FLAT_ENOUGH` and `MAX_SUBDIVISION`.
- The raster covers only the mesh's device bounding box intersected with the target.
- Each backend fills the shape with it as a nearest-sampled image at 1:1, so the *colour* is
  `pdf-render`'s — identical on both — and the *edge* is the backend's, antialiased as every
  other fill's is.

**The one thing given up** is that the mesh's own outer boundary is point-sampled rather than
antialiased. It shows only where a mesh ends *inside* the path being filled, which is a mesh
that does not cover the region its document asked it to fill. That is a small departure bought
with the removal of a larger one.

**This is shared code and that is the honest choice here, unlike ADR 0047's.** There the
argument for writing §11.3.5.3 twice was that Vello implements it independently, so the
cross-backend scene compares two readings. Vello has no mesh primitive at all — a second
implementation would be the same CPU-side loop written twice — so there is nothing to compare
and everything to drift.

## Result

**Three contradicted pages became agreeing**, and `agrees` went 812 → 815 with `contradicted`
86 → 83 and the corpus's incomplete count unchanged, so nothing was traded for a report:

- `mesh_shading_empty.pdf`, this list's one identified cause since the twenty-eighth session.
- `issue2948.pdf`, the same defect an order of magnitude louder — a moiré grid across a whole
  rainbow page, our green channel 3 to 48 where every reference reads 0.
- `issue18816.pdf`.

## Cost, which is the opposite of a cost

Measured with callgrind on `examples/callgrind_rasterise`:

| page | before | after |
|---|---|---|
| `issue2948.pdf` | **35.47 G** | **3.08 G** |
| `tensor-allflags-withfunction.pdf` | did not finish in a 15-minute callgrind budget (the handover recorded 22.0 G for its fill machinery alone) | **1.57 G** |
| `personwithdog.pdf` | did not finish in a 15-minute budget | **6.77 G** |
| a page with no mesh, `PDF20_AN001-BPC.pdf` | — | 0.82 G, unchanged |

**11.5× on the page that showed the defect**, because the old code's cost was per *piece* —
4096 fills of a few pixels each, every one of them compiling a `tiny-skia` pipeline — and the
new code's is per *pixel*, once. The sixteenth session's lesson has an inverse: a change made
for correctness that is also an order of magnitude faster means the old code was doing work
that was worse than useless.

## The spec track: §12.3, and fifteen `silent` rows where there were two

Document-level navigation — destinations, the outline, thumbnails, collections, navigators —
read as a family. **Not one of the twelve subclauses is implemented and nothing says so**, so
all twelve are `silent`, and the ledger's count of that status goes from 2 to 15.

That number is the finding. This project has been able to say "two `silent` rows are left"
only because clause 12's interactive half was `unreviewed`, and `unreviewed` and `silent` are
different admissions: one is *we have not asked*, the other is *we asked, and we owe it in
silence*. `CLAUDE.md` names "outlines, destinations, page labels" in scope explicitly, so this
is debt rather than a boundary — and the rows say what kind of debt each is, because a
thumbnail panel, a name tree and a SWF navigator are not one problem.

One row is worth reading before anything is built on it: §12.3.6's navigators are supplied as
SWF, which is what principle 5 excludes clause 13 *for* — but the exclusion list is closed and
this clause is in 12, so it is recorded as debt rather than claimed as excluded. Widening the
list is an argument somebody should make, not a thing to assume.
