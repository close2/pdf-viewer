# 2268 — The union's dropped stretch was a negative determinant rounded past the separation

Status: accepted for integration. Session 1717, 2026-10-23, the pixels slot of batch one hundred and
ten. Clause: ISO 32000-2 §10.7.4 and §8.5.3.3.2's non-zero rule; the row stays `departed` for its
other departures. Context: ADRs 2122, 2252 §1 and §5, 2253.

## 1. What was dropped, and why

ADR 2252 guarded a fault it could not explain: on thirteen marks of the moved pages at 1x —
`pr12564.pdf`'s nine, `issue14415.pdf`'s three, `issue12823.pdf`'s one — the union's boundary left
an accumulated row summing to something other than nothing, and the mark went to the library's
converter. Each of the thirteen was dumped as it reached `area::fill_union` and the smallest was
cut down, a piece and then a vertex at a time while it still failed: three triangles of
`pr12564.pdf`, nine vertices. Two of them meet at one point, and on the predicates' grid the
second's edge leaving it lies along the third's edge arriving at it, exactly; the first triangle's
edge crosses both of them at one point.

Where a crossing lies along an edge is `d₃ / (d₃ − d₄)`, two exact determinants converted to `f64`
by `union::approximate`, which took the value's high 64 bits and its low 64 bits apart. For a
negative value the low part is `2^64` less its magnitude: near `2^64` an `f64` is a step of `2^11`
apart, so the part rounded by up to `2^10` before `2^64` was taken off again, and a determinant of
`−652 948 884 480` came back a part in `10^9` off. The two crossings of one point read `2.3 × 10⁻⁹`
apart along the first edge, past `SEPARATION`'s `10⁻⁹`, so one of them was taken as a starting point
for the third triangle's walk; the order the rounding gave them was wrong, and the walk began one
winding off. No edge of the third triangle bounded anything, its walk still returned to the winding
it started from, and the rows it spans did not close.

The hypothesis the brief carried — a crossing record whose two stretches meet at a vertex the walk
visits once — did not hold: the bookkeeping was right and its input was not.

## 2. Decision

- `approximate` splits at bit 32: the high part of a value under `2^78` is under `2^46` and the low
  part under `2^32`, each converts exactly, and the sum is the one rounding — the nearest `f64`,
  which is what the `as` conversion of the whole `i128` gives, at the cost the two-part conversion
  was chosen for. `a_determinant_converts_to_the_nearest_f64_whatever_its_sign` holds that on ten
  thousand values of either sign, and `three_pieces_meeting_at_one_point_leave_a_closed_boundary`
  holds the nine-vertex mark's boundary closed at every point and in every row and draws it through
  `fill_union`; both are red on the split at bit 64.
- **The rows' guard is an assertion.** A boundary the walk returns closes by construction, so a row
  that does not close, and a walk that does not return to its winding, are defects: a build with
  debug assertions stops on either (`debug_assert!` in `area::fill_union` and `Union::settle`,
  planted and seen to fire). A release build declines such a mark to the library's converter, as it
  did, and that is not loud: nothing between `scan::mask_fill` and the host carries a refusal, and
  making one is `scan.rs`'s signature and every caller of it, which another slot of this batch owns.
  It is left to a round that owns the scan converter, and the cost is stated: a defect of this kind
  reaching a release build draws the mark at the library's sixteenths, silently.
- **What stays declined is degenerate.** After the fix the moved pages at 1x and 4x decline three
  marks, every one by the walk's entry rather than its rows: `annotation-fileattachment.pdf`'s push
  pin at 1x and 4x, a path that retraces itself and so states every one of its pieces twice — every
  point of every upright edge lies on its twin, and no ray can start the walk beside it — and one
  of `inks.pdf`'s at 1x, a piece four and a half units long and a hundredth tall whose two upright
  sides lie along other pieces' edges. Deferring such a contour until a crossing enters it was tried
  and enters none: the twins' crossings come in coincident pairs, which `SEPARATION` refuses as
  starting points. The module comment names both.

## 3. The pixels that move

The thirteen marks, which the library's converter drew, are drawn by the union: `pr12564.pdf`,
`issue14415.pdf` and `issue12823.pdf` move. ADR 2269's three changes to the pieces land in the same
re-take, so the pages and their figures are its §4.
