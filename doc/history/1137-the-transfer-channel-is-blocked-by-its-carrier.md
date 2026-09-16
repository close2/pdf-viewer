# 1137 — §11.7.5.2's transfer channel: agreement is solved, the per-mark carrier is not

2026-09-16. Files: `crates/render-raster/tests/transfer_edge.rs` (new), `doc/adr/1125-…` (correction
section), `doc/todo/13`, `doc/conformance/ledger.toml` (§11.7.5.2 note), this file. Shared working
tree: `pdf-model/soft_mask.rs`, `pdf-syntax/{crypt.rs,tests/encryption.rs}` and `pdf-transform/redact.rs`
were dirty from siblings and are none of mine. No pixel moved. §11.7.5.2 stays `partial`.

## The contract, and why the build stays deferred

Owned `render-cpu`, `pdf-render`, `raster`; contract §11.7.5.2's antialiased-edge channel 1118
deferred. The design (ADR 1125) is right and unchanged. I set out to build it in both backends and
found 1118's *deferral reasoning* wrong in premise and price — the corrected finding is the round's.

- **`render-raster` is not `tiny-skia`.** It is the `raster-gpu` **wgpu compute** backend; only
  `render-cpu` uses `tiny-skia`. `QuorraRasterizer::rasterize` returns pixels to the CPU and already
  runs CPU-side passes over the read-back (`resolve_grey`, `crop_to_page`, `impose_within`).
- **Agreement is achievable, so it is not the blocker.** The index §11.7.5.2 chooses is a pure
  function of geometry and opacity — colour-independent — so both backends apply the *identical*
  final map to the read-back and agree by construction. No GPU rewrite; no "second rasterisation pass
  in each backend" threaded through compositing.
- **The blocker is the per-mark carrier.** The function rides on the leaf `Command::Fill`/`Image`
  marks (groups do not carry it): **203 `Fill` + 69 `Image` = 272 sites across eleven crates**,
  including `pdf-model` (a sibling this batch), `render-gpu`, viewer crates, `test-scenes` — none
  owned here, and not landable backend-by-backend. The one carrier that avoids the sites, a
  `DisplayList` side-table keyed by top-level position, cannot index a mark nested in a group, so it
  drops the grouped case in silence (trap 5). So neither a one-backend change (forbidden) nor a
  flat-only shortcut (principle 1) is admissible: deferred, with corrected pricing.

## The fixture and gates

Population unchanged (1118's census: 13 state `/TR`|`/TR2`, one real — `issue6931_reduced.pdf`, fully
opaque; no corpus witness for the edge case; no document added this batch). Planted the raster half
of the fixture, the parallel of 1118's CPU one: `render-raster/tests/transfer_edge.rs` measures that
the raster backend draws the same pre-composite edge value (`blend(transfer(object), backdrop)`, gap
of half a unit to the clause) **and** that the two backends agree on it within `headless_quorra`'s
bar — the reason a one-backend fix is forbidden and the witness flips both together when the carrier
lands. Three tests, all pass (lavapipe software adapter).

**Gates**: test+docs only, no pixel code changed, so the corpus/oracle walks are the merge's, not
this round's. Exit codes in the report.
