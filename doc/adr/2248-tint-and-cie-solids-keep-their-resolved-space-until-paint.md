# 2248 — Tint and CIE solids keep their resolved space until paint

Status: accepted for integration. Session 1707, 2026-10-23.
Context: ADRs 2198, 2222, 2234; ISO 32000-2 §§8.6.6.4, 8.6.6.5, 11.7.5.3.

## Decision

A `Separation` or `DeviceN` colour (with §10.8.3's simulation and `/All`) and a `CalGray`,
`CalRGB` or `Lab` colour are kept as their resolved space and operands until a mark consumes
them, and converted there under the current target, intent, black point and pair. §8.6.6.4 has
the tint transform called "[d]uring subsequent painting operations", and §11.7.5.3's first
bullet gives a CIE-based colour painted into a group "the current rendering intent in effect in
the graphics state at the time of the painting operation". The colour converted at selection
was neither: a later `ri`, `/RI` or `/UseBlackPtComp`, and a Form group the colour is inherited
into, did not reach it.

`SolidSource` gains one variant, `Space`, holding an `Rc` of the space and its operands; the
two bounded pre-alpha memos and their complete-context key are unchanged. No new cache, no
second source enum, no invalidation flag. `Conversion::paint` is the same `paint_beside` call
selection makes, so an unchanged context paints the byte the selection painted. `sc`/`scn`
move the space they already clone into the source; `cs`'s initial colour clones it once.

The source is compared by allocation (ADR 2249 has the argument and its calibration). Pattern
binding keeps precedence; `/None` paints nothing in any context and is not kept.

## What remains outside

`Indexed` over a base other than an immediate `ICCBased`, and plain `DeviceGray` and
`DeviceCMYK` solids, keep their selection-time colour. One is demonstrated wrong by a probe:
`0 1 0 0 k` inherited into a `DeviceGray` group paints magenta [236, 0, 140] where §10.4.2.3
gives grey 105 and a cold selection inside the group gives 105. It is not built here because
`k` is the hottest selection in print files and wants an inline source and its own cost.
