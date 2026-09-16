# 1135 — A seventeenth empty head, and §46's residual filter is excluded, the ask re-pointed

2026-09-16. Files: `crates/render-raster/{examples/image_residual.rs (new), tests/corpus.rs}`,
`doc/QUORRA_FEEDBACK.md` §46, this record. No ADR: no reading of the standard was decided — a
measurement excluded a stated cause and named the path the ask belongs on. Every other `git status`
path (`pdf-model/src/{transfer.rs,structure.rs,content/ext_gstate.rs}`, `pdf-transform/…`) is a
sibling's; one broke my build for one window (an undeclared `mod transfer`) and cleared on its own.

**Oracle head empty a seventeenth round** — `1962 pages in 54.3s`, `agrees 1012, contradicted 46,
ambiguous 835`, the "ambiguous, undiagnosed, and furthest from the nearest reference" line printed
with nothing under it. So the contract was the eight `render-raster --test corpus` differers.

## The eight re-confirmed, and §46 sharpened

`render-raster --test corpus`: `958 compared, 943 agree, 8 differ, 7 refused, 16 not comparable`,
identical to 1129/1123. The 8, each re-confirmed by its gate line: `issue20232`, `issue21068`,
`issue15150` (§45 clamped winding); `22060_A1_01_Plans` (§46); `pr12564` (§47); `issue269_2` (§48);
`issue19083` (§24c); `issue2177` (standing curve floor). None closable on our side.

**§46 is the one converter/reduce ask without a document-free reproduction, so this round built one
and it excludes the stated cause.** `examples/image_residual` reduces stripes, thin rules and a
continuous-tone grating at every ratio — pure residual `1.0..=2.0`, two-stage floor-then-residual,
and 22060's own 6→1.15 and 3.44→1.72 geometry — and cpu vs raster hold to **≤0.10%** on all (worst
0.27%, the magnify control's edge AA). The ordinary reduce+residual filter §46 named is ruled out.
22060's four heavy images are each a `DCTDecode` `DeviceGray` scan carrying an `/SMask`, so each
takes `render_raster::scene`'s deferred `AtDeviceScale` arm (§11.6.5.2's mask on a device grid), not
the ordinary reduce the reproduction drives. §46 and the `corpus.rs` note are re-pointed there.

## State of robustness

Oracle: 1012 agree / 46 contradicted / 835 ambiguous of 1962 (also 3 our-geometry, 2 reference-
geometry, 46 not-comparable, 18 no-render), head undiagnosed empty. `render-raster`: 943 agree / 8
differ / 7 refused / 16 not comparable of 958. `text_extraction`: overall 99.3% (24608/24787)
words-in-bounds, verdict 99.69% (11096/11131), 494/503 docs fully in bounds, ratchet 8266/8266.

## Gates (under the lock, one at a time)

`oracle` exit 0 (1012/46/835, head empty); `render-raster --test corpus` exit 0 (943/8/7/16);
`raster_golden` exit 0 (held 974, moved 0); `text_extraction` exit 0 (99.69%). Tier 1 on
render-raster: fmt, clippy, conformance exit 0.
