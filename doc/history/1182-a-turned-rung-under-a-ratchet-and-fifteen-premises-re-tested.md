# 1182 — A turned rung under a ratchet, the rasteriser record rewritten, and eight premises expired

**§10.7.5's ladder is a gate.** `render-cpu/tests/stroke_width.rs` gained 96 turned rungs — 45°,
three device widths, two scales, eight placements, `/SA` absent and `/SA true` — the population
ADR 1082 moved onto `area.rs` and the one ADR 1189 found had gone 47× better unreported. Worst
deviation **0.0038** of a device pixel, reproducing ADR 1189 section 3 to the digit; the axis-aligned
ladder reads **0.0078**. Both tolerances are ratchets now, the measurement doubled — 0.008 and 0.016
against 0.05 — and both tests print their population and worst rung. Calibrated by widening the
stroke and the expectation by 0.1 device pixel: both fail (trap 13).

**ADR 1201** supersedes ADR 0002's record; all three claims of its revisit note hold. Page one is
the device's, the oracle's scan conversion is `area.rs`'s, and three rasterisers put the shipped one
at a corner of the comparison. `tiny-skia` stays, for geometry, the stroker and dasher, the shaders,
the surface and blitter — none of which decides a coverage. **ADR 1202**: ADR 1134's closing sentence
grounds the §7.6.5 refusal on a fact its own Decision 2 manufactures, the standing ground being the
crate-graph blocker and A66's trigger; the census is not circular, so `ledger.toml` is untouched.

| ADR | premise | verdict |
|---|---|---|
| 0348 | no compiled-in face has an Arabic glyph | holds — 14 faces, no shaper, broker is no glyph source |
| 0803 §1 | no output-resolution path exists | **expired** — `viewer-host::printing`, GTK print-to-file |
| 0821 | synthesising `/Info` would be authoring | **expired** — `Edit::SetInformation` writes Table 349 |
| 0847 | the vfs worker needs no call the viewer does not | holds — write verbs probed confined |
| 0918 | no host offers a port for a document's references | holds — `offer`'s one description is a face |
| 1012 | `preserve`/`derive`/`supply` recognised, not applied | **expired** — built; shape qualifier left |
| 1057 §4 | `/FixedPrint`'s printing half waits on a print path | **expired** — ADR 1180; row `implemented` |
| 1069 | this device is a screen, no black-generation step | **expired** — ADR 1173 retires the ground |
| 1107 | no first page states a non-isolated group under a mode | **expired** — ADR 1170 synthesises one |
| 1108 | ISO 19444-1 sections 6.4–6.7 are not on this disk | holds — the preview stops in 5.7.1 |
| 1113 | a per-pixel shape channel is not worth its cost | **expired** — `transfer_channel.rs` is it |
| 0653 | `/GEO` sits only in clause 13's annotations | holds — the exclusion is closed and unamended |
| 0660 | #307's `shall not` is addressed to a writer | **expired** — this tree writes four name trees |
| 0705 | no corpus page shows dots on its zoom path | holds — and no instrument can observe it |
| 0761 | the Windows verification needs the owner's machine | holds — `doc/todo/50` has no second trace |

**Eight of fifteen expired — a finding for the batch after.** The sweep ranked these *lower*
confidence than the sixteen it wrote up and this half expired at the higher rate, which measures the
ranking rather than the tree: a premise is cheap to state and nothing re-reads it, so age predicts
expiry better than confidence does. The eight builds are `doc/todo/65`'s new last section; and
ADR 0821's `/Info` decision is its section 9, not the section 4 the list names.
