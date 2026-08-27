# 0701 — The stroke width is quorra's to resolve

**Status.** Accepted — the §4.5 amendment the project owner approved on 2026-08-27,
asked "what is the downside?" and told it plainly: a clause's arithmetic now lives in
two trees. This file is that cost written down, and the reason it is worth paying.
Quorra's side is their ADR 0085; the plan both serve is their ADR 0084.

## What changed, on this side

`render-quorra` stops resolving a stroke's device width per frame. The width crosses
the boundary as the file stated it — the command's own space — together with §10.7.5's
`adjust` flag, and quorra's encode resolves §8.4.3.2's zero-width rule and the
adjustment per placement, with arithmetic mirrored statement for statement from
`pdf_render::Stroke::device_width` and `Transform::max_stretch`. Dashing and
§8.5.3.2's degenerate-subpath splitting stay here (the split is the remaining
view-dependence in a stroked scene, and it moves as its own change because it carries
cap semantics with it). `render-cpu` — the oracle — is untouched: it still resolves
through `pdf-render`, which is exactly what makes the cross-lane pixel gates a
continuous comparison of the two implementations.

## Why it is worth a duplicated clause

A scene that states a device width dies with the magnification, and everything in
quorra's ADR 0084 — scenes that survive zooming, retained records replayed under a new
affine, eventually the walk on the device — requires every retained command to be true
at every viewport. The stroke width was the last entry in the scene contract that was
not. The measured chain it unlocks is that ADR's: the worst page's zoom step from
~95–110 ms toward the device-bound floor.

## The containment for the duplication

- The formulas are mirrored by statement, not re-derived, and each side's copy names
  the other.
- The conformance ledger's clauses (§8.4.3.2, §10.7.5) keep their home here — this
  tree's `device_width` remains the cited implementation and the oracle's path — and
  quorra's copy cites the same clauses as a mirror.
- Divergence is a failing pixel gate, not a silent difference: the lanes compare
  continuously, and the substituted hairline (the one case that differs by
  construction, an ulp of `(1/stretch)·stretch` against an exact `1.0`) is inside the
  relaxed contract (quorra's ADR 0082) and measured at zero moved pixels on the
  windowed A/B.

## Held by

Quorra's whole suite (612) with its stroke gates restated in scene units; this tree's
sixty `render-quorra` tests and the full workspace, unmodified, against the amended
quorra — the pass-through changed no pixel anywhere a gate looks. The commit of this
side waits on the quorra pin reaching ADR 0085, exactly as the lock discipline
requires.
