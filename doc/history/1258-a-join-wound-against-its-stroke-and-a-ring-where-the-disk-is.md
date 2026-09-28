# 1258 — A join wound against its stroke, and a ring where the disk is

Batch thirty-nine, corpus slot. No ledger row moved; no page moved list. `render-raster --test
corpus` 946 agree / 5 differ / 7 refused / 16 not comparable before and after.

## The five differing pages
- `ContentStreamNoCycleType3insideType3.pdf`: its paragraph was wrong (it blamed tile seams). At
  4× raster draws radial slivers through thick stroked glyphs, 4% short at every ink-ladder rung.
  §8.4.3.2's "all points whose perpendicular distance from the path in user space is less than or
  equal to half the line width" decides it for the oracle. Cause, read in raster's `stroke.rs`: `join_at`
  winds the join against the quads when `cross > 0`, so a join overlapping a quad cancels it and a
  shared boundary pixel reads `|a − b|`. Fixtures against the distance set computed in Python: a
  `16 w` hook drawn one way has slivers and drawn the other way is solid; a 1 w circle is 187.40
  clockwise against 188.94 anticlockwise (truth 188.50). `QUORRA_FEEDBACK.md` section 54.
- The same fixtures convict the **oracle**: a closed curve stroked wider than its diameter is a
  ring (radius 5 at `30 w`: hole of radius 10). `tiny-skia` 0.12's `PathStroker` emits the inner
  offset as a reversed contour; its own `stroke_path` leaves the hole (scratch crate, centre alpha
  0). The fix site is `render-cpu/src/lib.rs`, 1255's this batch: handed over, not edited.
- `issue20232` (+32.2% → +4.4% up the ladder), `issue15150`, `issue19083`, `issue2177`: reasons
  re-read and hold (sections 45, 45, 24c; the edge floor). Numbers refreshed.

## The seven refused
Four before the scene (three CIE-based, one four-component group) are section 43's and raster's
`GroupSpec`/`MaskKind` still carry no conversion or second body; three at the device are budgets
and a capability (sections 40, 44, and the cycle by design). None can leave.

## `issue6127.pdf`
Not newly incomplete: 61 = ceiling, and it is ADR 0433's §9.7.5.2 population ("The Identity-H and
Identity-V CMaps shall not be used with a non-embedded font"), held by `silent_fonts.rs`. The
brief's premise was wrong; no batch-38 change made the report.

## `NOT_COMPARABLE`
Eleven passwords/encryption, five no first page: every reason holds.

## Paperwork
`render-raster/tests/corpus.rs`'s list comments rewritten as what is (departure chronicles cut to
the ADRs); a dangling sentence fragment removed.
