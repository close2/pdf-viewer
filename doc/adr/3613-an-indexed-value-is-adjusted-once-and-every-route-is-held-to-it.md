# 3613 — An `Indexed` value past `hival` is adjusted in one place, and every route is held to it on three backends

Session 2228. Status: **accepted**. Context: round 2224's record, which listed "an `Indexed`
sample past `hival` painted black where §8.6.6.3 adjusts it to the nearest index" as left, from
a helper's reading of `pdf-model`'s `image/unpack.rs` (`palette`'s `unwrap_or(BLACK)`); ADR 3591
(the errata are searched first: none of `doc/errata/` names §8.6.6.3). Clause: ISO 32000-2
§8.6.6.3.

## Context

> The index value should be an integer in the range 0 to hival . If the value is a real number,
> it shall be rounded to the nearest integer (0.5 values shall be rounded up); if it is outside
> the range 0 to hival , it shall be adjusted to the nearest value within that range.

The sentence names no route. Every route this tree has reaches one function, `pdf_colour`'s
`ColourSpace::entry_of`, which rounds and clamps to `high` before reading the table: the colour
operators (`content/colour.rs`'s `Indexed` arm), every conversion of a value in an `Indexed`
space (`to_rgb`, `to_cmyk`, `cie_xyz`), the mesh shadings' vertex colours (`mesh.rs`'s
`Components::read`), and the image unpacker, whose per-depth table (`palette`) holds an entry for
each of the `1 << bits` levels the depth can write, each painted through `entry_of`. The
`unwrap_or(BLACK)` the helper read is past the table's end, and no sample of at most eight bits
reaches it. The devices draw a display list whose colours and image samples are already the base
space's, so neither `render-raster` nor `render-gpu` looks an index up.

## Decision

1. **Nothing to fix; the reading is held instead.** `tools/conformance/test-support/
   indexed_past_hival.rs` writes one document per route — an image at 8, 4, 2 and 16 bits, one
   under a `/Decode` that maps below zero, one with a `/Matte`, an inline image, `scn`/`SCN`, the
   base of an uncoloured tiling pattern, a free-form mesh, an axial shading's function output,
   and an image inside a luminosity soft mask — each over a two-entry table whose ends are two
   colours, with the colour the clause gives each patch.
2. **Three backends draw them**: `pdf-model`'s `colour_suite` (render-cpu, one test per route),
   `render-raster`'s `indexed_past_hival_spec_audit` under its three coverages, and `render-gpu`'s.
   All agree with the clause, so no golden page moves.
3. The palette's comment says why its default is unreachable, under the silent-repair campaign's
   `not a repair:` marker.

## Consequences

- A table shorter than `m × (hival + 1)` bytes reads its missing entries as 0 in `entry_of`; that
  is a malformed table, not an index outside the range, and is left to the silent-repair campaign.
- A JPEG 2000 image's palette indices (`pdf-sandbox`'s `decode.rs`) are clamped to a byte and then
  meet the same table; no document here exercises that route.
