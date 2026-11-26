# Round 2265 — option 3 is priced: flattening by device pixels alone below a size

A pricing round in `pdf-viewer-audit` on main `d94ef46b3`, answering A1073's "price 3 next".
Nothing in the tree draws differently; the prototype is a patch beside the round's files.

## What was found

The relative flatness bound (`RELATIVE_FLATTEN_TOLERANCE`, raster's ADR 0044) is raster's alone:
`flatten.rs` and the compute lane's WGSL. The CPU oracle flattens to an absolute 1/256 px (1/20 px on a
folding stroke), `render-gpu` (vello 0.9) to an absolute 0.25 px, and raster's device coverage lane
does not flatten. `render-cpu`, `pdf-model`, `viewer-host` and `render-gpu` do not depend on
`raster-gpu`, so `raster_golden` and the strips test cannot move under any setting of it.

## What was measured

- Prototype: `QUORRA_FLATTEN_DEVICE_ONLY_BELOW=<px>` on both raster flatteners, off when unset;
  ADR 3761 states its lines, and it is not in the tree.
- Type 3 page, three interleaved turn children under one clock hold: turn 2386–2628 ms off,
  896–1119 at 2 px, 941–1524 at 8 px; callgrind 62.3 G → 35.0 G, `exact_areas` still 64 %.
- Corpus first pages through raster, against a baseline, control 0 of 957: 522 move at 1 px,
  593 at 2 px, 616 at 8 px; by 8 levels or more 5, 11 and 147. Text pages, 1–6 levels a rim pixel;
  at 2 px 379 of the 593 move away from the CPU oracle, 21 toward it.
- Two raster unit tests fail at 2 px (a 0.5 px dot's ink, the piece counts).
- `issue18032` draws a stray dark pixel inside a white glyph on main; reported, not fixed.

ADR 3761 has the tables; Q1086 asks the owner, recommending the relative bound stays.

**Ledger.** 10.7.2 (`implemented`) and 10.7.4 (`departed`) re-read, unchanged: nothing landed.

**Gates.** The tree's code is main's: the round adds three documents. On the prototype: fmt 0; clippy
`-D warnings --all-targets -p raster-gpu -p render-raster` clean in `raster-gpu` (the sweep example,
not kept, was not); `raster-gpu --lib` curve tests with the switch at 2 px 36 passed, 2 failed (above);
turn walk 101, the Type 3 page below its band; launch walk 101: two memory rows
below their bands as on 2263's and 2264's walks, and two page turns past theirs (BPC 15.3 ms at
load 31, WTPDF 18.3); the control without the switch, at lower load, turned them inside (ADR 3761). `cargo test -p
conformance` 101 on `bounded.rs`'s self-test alone, under load,
which passes run by itself; every other test 0.
