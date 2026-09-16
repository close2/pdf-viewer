# 1133 — The bare constant is opacity at every shape, and now at a fractional one

2026-09-16. No ADR (a calibration, not a decision beyond the clause). Files:
`pdf-model/tests/transparency_groups.rs` (+2 tests), `pdf-model/examples/group_shape_census.rs`
(a fourth knockout column), three ledger rows (§11.4.6, §11.3.7.2, §11.6.4.4). `render-cpu`,
`pdf-render` and `raster` needed no line — the residue was already carried.

**The residue resolved: `ca`/`CA` are read (`ext_gstate.rs`) and applied as opacity, not shape, at
every knockout site.** §11.3.7's three plants (bare command not `Shaped`, `ca`/`CA` never read) are
absent. A bare translucent knockout element — a solid at `ca` < 1, no mask — has its shape *be* its
coverage, so it goes to the backend unwrapped, drawn under `Compose::Knockout` = tiny-skia `Source`,
which weights the immediate backdrop by `1 − f` (the shape) while the paint carries the constant as
opacity. That is §11.4.6 exactly; no second channel is owed. Masked, shading and image-opacity
elements arrive `Command::Shaped` (two draws); only `SampleAlpha::Both`, a non-isolated group as an
element, and `/AIS` both ways stay reported.

**The gap was calibration, not construction.** The stated-shape route had a pixel (opacity via
`/GM`); the bare-constant route had only the flag (`a_knockout_group_of_opaque_marks_carries_its_shape`
asserts `alpha_is_shape`, no pixel). Two tests close it. At a **half-covered edge** over an opaque
backdrop the bare `ca ½` mark is `(191, 64, 128)` — the value the mask route reaches by two draws —
where the constant read as shape (bare draw source-over) gives `(191, 0, 64)`. At shape 1.0 the
topmost element knocks the one below out whole: overlap `(128, 128, 255)`. Both derived from §11.4.6;
`render-cpu`'s hand-built fixture holds the f=1 overlap, these add the reading from `/ExtGState` and
the fractional weight. Calibrated (trap 13): planting `Knockout` → `SourceOver` fails both, reverted;
`render-cpu` diff clean.

**Census (trap 8), `group_shape_census` gained a bare-translucent column.** doc/pdf.js 963 first
pages: 33 knockout groups on 18 pages, 5 stated shape, **13 bare translucent**, 0 own-backdrop, 0
refused. doc/corpora 487: 6 knockout on 5 pages, **6 bare translucent**, 0 stated, 0 refused.
`knockout_isolated_overlap.pdf [2 fill]` is the fixture living in the corpus; opaque groups
(`derivable true`) are excluded, only paint alpha < 1 counts.

**Movers: none.** `raster_golden` held 974, moved 0. Oracle 1012 agree / 46 contradicted (all held
by named groups) / 46 not comparable — unchanged from 1099's 1012/46. **Rows: none moved status**;
the three gained the two tests and a sentence, and §11.4.6's hardcoded census counts were replaced by
the census command (the counting rule).
**Gates** (under the lock): `raster_golden` 974/0 (exit 0); `oracle` 3 passed (exit 0);
`render-raster --test corpus` agrees (exit 0); `pdf-transform --test gate` 71.9 pages/s (exit 0);
`conformance` 259 passed (exit 0). Tier-1 `clippy --workspace` red on a sibling's untracked
`render-raster/examples/image_residual.rs` alone; my crates clippy clean.
