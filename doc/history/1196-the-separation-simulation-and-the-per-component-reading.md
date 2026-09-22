# 1196 — The separation simulation is built, and §8.6.6.5's departure has a route

The colour round of batch thirty. Three rows briefed; two moved, one narrowed.

ISO 32000-2 §10.8.3's four steps, in `pdf-colour`, under round 1195's control (ADR 1228). An
`NChannel` space naming a spot colourant now reads as separations — each spot through its own
`/Colorants` `Separation`, the process components together through Table 71's space — converted
to flat XYZ against a white matte and multiply-blended in that matte's own white. ADR 1229 has
the two readings the clause does not state: which separations a colour has, and what unit Table
136's multiply is taken in. `Reading` carries the preference through the parse chain,
`Conversion` to the image, shading and mesh routes.

## Rows

- **§10.8.3** `reported` → `partial`. Steps b), c) and d) executed; step a)'s page-scale half is
  not — it wants a plane per colourant, and a spot ink has none.
- **§8.6.6.5** stays `departed`, and the departure narrows from *the capability* to *the default*.
  `doc/questions/Q100` asks the owner whether a requirement executed under a host control counts as
  executed, recommends `implemented`, and its answer moves the row.
- **§11.7.2** stays `partial`, on one sentence instead of two. ADR 1230: "four components with no
  profile behind them" invited a false reading — such a group composites in ADR 0263's assumed inks
  — and what is refused is a space §11.6.6 excludes from being a group colour space at all. What is
  left is `MAX_PRESSES`, a budget.

## Numbers

- Callgrind A/B, one sitting, two binaries from one tree: 1 253 064 426 against 1 252 937 239
  instructions, **+0.010%**, ~2500 per interpretation, none per pixel.
- `raster_golden`: held 974, moved 0. `pdf-model --test corpus`: every ratchet at its ceiling.
- Trap 1: three real `/Colorants` documents rendered both ways. One moves — 579 pixels of
  1 127 859, worst channel 8 — and both pictures are right.
- `/SeparationInfo` is a literal in 9 of 90 765 files here, `/NChannel` in 1277, `/Colorants` in
  141. It is Table 400 in §14.11.4, not §10.8's at all.

**Found, not fixed** (§11.7.4.3's row and `content/overprint.rs`): §10.8.2's cyan-over-yellow
example draws green written with `k` and **yellow** written with the `Separation` spaces the
example names. NOTE 2 makes a reverting `Separation`'s current colour space *be* its alternate;
`content::colour::cmyk_tints` answers only for a literal `ColourSpace::Cmyk`. Also left: step a)'s
`ColorantTable` is an ICC tag read by nothing, and `doc/questions/Q100` is open.
