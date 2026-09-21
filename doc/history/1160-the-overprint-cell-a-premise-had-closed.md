# 1160 — The overprint cell a premise had closed

Batch twenty-four, branch `batch-1159-1164`. Rendering/ledger round.

## What the clause said

§11.7.4.3's first bullet conditions on "the current colour space and group colour space" being
`DeviceCMYK`, not on the device's; §11.7.4.2 puts the blending computations in the group's
components and the device's conversion after them; §11.7.4.5's NOTE 1 names a group whose colour
space is not the device's as where the transparent and opaque rules differ. ADR 0028 closed six
rows on the premise that every group here composites in the device's three components and wrote
its own expiry — a group whose blending space is `DeviceCMYK`. ADR 0262 built exactly that and
nothing re-read the rows.

One cell is owed, not the whole table: Table 146's first row under `OP true, OPM 1`. The spot rows
have no colourant to affect, §11.7.3's own `shall` closes the `Separation`/`DeviceN` rows, and
every remaining cell is `C_s`. §11.7.4.3's verb is `may`, so declining is permitted and is not
`inapplicable`; `CLAUDE.md`'s *done* decides between the two.

## What was built

Table 57's three entries are read with its own precedence, and the four tints a `DeviceCMYK`
operator stated are kept, because §8.6.7 puts the zero test "on the tint value defined within the
PDF file, before quantisation". `BlendMode::Overprint` carries which raster channels the bullet
leaves to the backdrop; `render-cpu` computes it beside §11.3.5.3's four; `render-gpu` and
`render-raster` refuse the list by name off one bool. §11.7.4.3's implicit group under a non-Normal
mode and §11.7.4.4's first bullet where the pair's alpha or mode is not the identity are reported,
each narrowed by a derivation (ADR 1158).

## Rows, and what was measured

§8.6.7 and §11.7.4.1 `inapplicable` → `implemented`; §11.7.4.3 → `partial`; §11.7.4.5's note
rewritten; §11.7.4.4 and the aggregate keep `partial`. §10.7.5 untouched. `raster_golden`: 966 of 974 first pages moved on the list digest and **none on its raster** — one
new `DisplayList` field in a `Debug` rendering — so it was regenerated. One of the 966 moved its
reports too, and that page is the finding: `issue12798_page1_reduced.pdf` states a `DeviceCMYK`
page group, `/OP true /op true /OPM 1` and `/BM /Multiply`, which is §11.7.4.3's last paragraph.
It says so now, so `MAX_INCOMPLETE` rises 61 → 62 and `AMBIGUOUS_NON_ISOLATED_POSTER` empties;
oracle green. Callgrind, planting the added calls away: the default path costs **0.032%** of one
page's interpretation, from 0.30% before `overprint_blend` was split hot from cold (ADR 1158 §3).
The census ADR 1158 §4 asks for is `doc/todo/23`'s and is left owed.
