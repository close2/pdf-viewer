# Q473 — May the line lane's endpoints be mapped on the device, so that a zoom sends only a matrix?

Source: ADR 1992, the evaluation of the owner's ask of 2026-10-19 ("keep the page's intermediate
representation resident on the GPU and have a compute stage re-rasterise it at the new transform, so
a zoom sends only a matrix"), and `doc/todo/75` item 2.
Status: **open** — answered when `A473-…` exists beside this file.

## Why it needs the owner

On the owner's witness a zoom step is 287–301 ms, half of it host encode and transfer, and 95 % of
the bytes it sends are the line lane's instances: 4 398 932 lines at 60 bytes, written by the host
from **device** endpoints the host computes per view (ADR 1992 section 2). Keeping them on the device
across a zoom means writing them once in page space and letting `line.wgsl` map them by a view
uniform. That moves one computation from the host to the shader, and WGSL section 15.7.5 lets a
driver reassociate and fuse it: a line's device endpoints may then differ from today's by an ulp,
and from one adapter's to another's. Two bars meet there:

- **The oracle**, which is the processor and stays the processor. The line lane is already held to
  it by a bound, not by bytes, and raster's ADR 0082 relaxed the device lanes to a close best effort;
  the compute lane's fills already map their outlines on the device under the same section and are
  held to one level for glyphs (ADR 1420). This would put the line lane's endpoints in that class.
- **ADR 1877's kept frame equal to a fresh one**: kept by construction, since both would map on the
  device.

What changes is that a line's pixels would no longer be the same bytes on two adapters, nor today's
bytes on this one — a step of the kind the discipline says needs its own ADR and an arm that shows
which pages moved and by how much.

## Recommendation

**Accept it**, held to the line lane's existing bound against the processor and to a one-level
tolerance between the kept and the fresh device frames across adapters, as the compute lane is. The
saving is the witness's step from 287–301 ms towards its replay's 127–141 ms once item 3 builds the op
list once, and nothing on any of the eleven turn rows, whose steps are not lines.

## What the tree does meanwhile

Lines stay mapped on the host, so their bytes are today's. What is built (ADR 1993) keeps a retained
encode's streams on the device for a replay at the same view, which moves no pixel; items 2–4 of
`doc/todo/75` wait for this answer.
