# 1261 — A stroke its matrix collapses is its set's image, and the walk has a floor

Batch forty. §10.7.4 `partial` → `departed` (one pixel: a shape with no extent in either axis,
ADRs 1060 and 1360); §10.7 `partial` → `implemented`. `doc/todo/65` bucket 3 is empty.

## The residue (ADR 1360)

A matrix of rank exactly one (`ad = bc` in `f64`) maps the plane onto a line by a functional; the
stroke's set is a union of convex parts, so its image is one interval per subpath or dash, computed
from path, width, caps, joins and dashes in `pdf-render/src/collapsed/stroke_image.rs` — no stroker.
Fixtures: a diagonal segment at `5 w` is row 49 from `20 − 2.5/√2` to `60 + 2.5/√2`; a rule the
matrix makes a point is five pixels at `5 w`; planted away both fail. A line across the axes is the
band of one device pixel (`1 1 1 1 0 0 cm`: `70√2`). A point stays ADR 1060's.

## The cost (ADR 1359)

One chain is one strand (−1.86% on `issue19802`), each edge's row found once (−1.34%), a folding
stroke's other stretches to the stroker at sixteen times its resolution with joins and caps turned
from exact tangents (−8.74% on `issue14415`). Against the tree before ADR 1341: +18.85%, +14.49%,
+14.55%, page 101 −2.52%. Not the target; the floor is measured: walk off, 19802 and 20232 draw
19% and 24% cheaper than that tree, and 99.4% of 20232's walked clusters part from the set. Folds
walked exactly cost +120% per draw on 14415: the sixteenth stays. Trace-once rejected (habit 47).

## Files outside the brief's list

`pdf-render/src/paint.rs` (doc), `pdf-model/src/content.rs` (one rename),
`pdf-model/src/content/report.rs` (doc), `pdf-model/tests/variable_text.rs` (singular `Tm` onto
`x = y` now draws its band), `doc/checks/fixed-documents.toml` (2883540, 7803534 no longer report the
matrix: a collapsed stroke is drawn; inks unchanged), `doc/state-of-play.md` (one sentence).

## Gates

fmt, clippy `-D warnings` (render-cpu, pdf-render, pdf-model), nextest 424 and 1679 pass,
conformance 0. Isolated on HEAD plus these files, under the lock: `raster_golden` 24 moved, all
`raster only`, all by the stretches, each diffed pixel by pixel (boundary sixteenths; one seam of
ADR 1348's closed), rows regenerated; `pdf-model --test corpus` 0; `render-raster --test corpus`
946/5 before and after; `fixed_documents` 0 after the two rows; oracle: verdicts identical to HEAD
(both fail on `ISO_IEC 15444.pdf`, a doc/ spec in the population, trap 43).
