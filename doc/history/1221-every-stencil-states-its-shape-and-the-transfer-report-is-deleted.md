# 1221 — Every stencil states its shape, and the transfer report is deleted

Date: 2026-09-22. Branch: `batch-1219-1224`, worktree `/home/AI/pdf-viewer-rounds`, five sibling
rounds (1219, 1220, 1222–1224). ADR 1279.

## What the brief said, and what the tree said

The brief asked for a shading's ramp and a tiling cell on §11.7.5.2's channel. ADR 1266 had
built both, and their fixtures assert the clause's value. The one reach of
`Unsupported::TransferFunction` left was a `SampleAlpha::Both` image whose source could not hand
back its shape, so that is what was built.

## Built

- A stencil under a soft mask keeps its pair apart on every route: a mask the device-scale route
  declines is decoded eagerly into a plane, a `/Matte` is no reason to combine (Table 144 counts
  it in a `/ColorSpace` a stencil may not have), and an `/SMaskInData` opacity is read into a
  plane. That last one had been silently dropped: `jpx_stencil` never read the channel.
- `Unsupported::TransferFunction` is deleted, along with its viewer sentence and the knockout's
  image refusal arm, which no page could reach any more.
- §11.7.5.2's overprinting paragraph, which no row had read: a mark whose special mode keeps a
  backdrop component is opaque for none of the device's three components.
- `pdf_colour::shading::Colouring`'s dead transfer, and the comments on it, removed.

## Numbers

Rows: §11.7.5.2 `partial` → `implemented`, §11.7.5 `partial` → `implemented`, §11.7 still
`partial` for §11.7.4. `raster_golden`: 974 held, **0 moved**, run twice. `render-raster --test
corpus` sits at its ceiling. Three new fixtures, each failing with its rule planted back. The
stencil page gives 255 255 255 at a painted cell of opacity 0 where the product gives 0 0 0. It was
rendered and looked at, and both shipping backends agree on it pixel for pixel. `render-raster`
refuses the overprint page by name, and the CPU draws it as derived.

## Handed over

§11.3.7, §11.3.7.2, §11.3.7.3, §11.4.4, §11.4.6 and §11.7.4.4 still name the `Both`, codec and
`/Matte` residue this closes. `doc/todo/13` is kept, because `CLAUDE.md` cites it.
