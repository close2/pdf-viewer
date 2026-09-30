# 1291 — Two backends compared draw the same frames, and the one level is the device's

Batch forty-five, raster slot. No ledger row moved (the contract named none; §10.7.4 and
§8.5.3.3 stay `implemented`). ADRs 1419, 1420; feedback section 58 appended. `crates/render-cpu/`
was not opened.

## The 4× residue (ADR 1419 section 1)
- Instrumented in an export with its own target directory: each atlas insert, probe and settle
  per backend, page names between frames, outline ids stripped. First differing insert: the first
  glyph of `issue12810.pdf`, which is refused at 4× by the frame budget after committing inserts;
  the gate drew the one-threaded arm only when the fanned-out one had drawn, so from the next page
  (`issue12823.pdf`, the survey's first name) the two atlases differed. Artefacts were a second
  draw on one backend only, the same shape.
- Brief's three hypotheses (pending bound, repack count, commit order): none. Fix in the gate:
  both backends draw every page, refusals compared by their words, artefacts from the judged frame.
  Insert, probe and settle logs identical over the walk; 176 → 0; the list held at every scale.
- `encode_threads.rs`: a refused long page then a text page, 1–64 threads, same refusal, counters
  and bytes; its counters differ from a fresh device's (the inserts stand).

## The shared `OnceLock` (ADR 1419 section 2)
- `winds_two_values_as` removed; always the outline's own flattening. Test outline: an arc bulging
  1.5 units with a same-wound rectangle inside the bulge; at a tenth of its size the arc flattens
  to its chord. Asked placement-first, the old code fixed `true` against the outline's `false`.

## The tie (ADR 1420)
- Both lanes round half up the same `f32`; the forms differ only at `0.5 − 2⁻²⁵`. Scratch variants
  of the processor's arithmetic: nested fused transform takes the two pixels both drivers share;
  a fused deposit takes RADV's two exact-half pixels (127/128) and makes llvmpipe's. The two
  drivers' compute lanes differ from each other on 4 pixels. WGSL section 15.7.5 permits fusion and
  reassociation: held at one level, reason in `compute_lane.rs` and `compute.rs`.

## Files
`crates/render-raster/tests/corpus.rs`, `raster/crates/raster-gpu/src/resources.rs`,
`encode/parallel.rs`, `tests/encode_threads.rs`, `tests/compute_lane.rs`, `src/compute.rs`
(module comment), `doc/QUORRA_FEEDBACK.md`, `doc/state-of-play.md`, ADRs 1419, 1420.

## Gates
Corpus 1× 959/2, 4× on all three lanes, each thread count 0; `headless_gpu`; tier 1 on the four
crates and `conformance` (exit statuses in the report). `raster_golden` not run, not regenerated.
