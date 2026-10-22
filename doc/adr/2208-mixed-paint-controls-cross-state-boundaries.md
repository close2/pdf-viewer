# 2208 — Mixed paint controls cross state boundaries

Status: accepted, 2026-10-22.

## Context

Section 11.7.5.3 applies the current black-generation and undercolour-removal functions
when an elementary RGB object is painted into a CMYK transparency group. Correct
isolated colour conversion and complete cache-key equality do not alone establish
that every consumer uses the restored graphics state.

## Decision

`crates/pdf-model/tests/mixed_paint_context.rs` exercises fill paths, stroke paths,
fill-and-stroke text and one shared indirect stencil within the same interpretation.
It changes conversion functions after selecting sources, paints with changed sources
and alpha under q/Q, then returns to the warmed enclosing state. A second sequence
isolates reuse of the same indirect stencil through those changes.

Each region is compared with a separately interpreted page that selects its colour
after establishing the complete state for that region. Both source orders are used.
Every consumer must produce visible pixels; an arithmetic colour sample independently
anchors the BG/UCR expectation. These controls hold behaviour, not the number of cache
slots, the order of internal calls, or a particular caching implementation.

Calibrate the instrument in an isolated export with a missing context comparison and
a stroke consumer bypassing paint-time conversion. The normal tree must pass while
the incorrect variants fail assertions. Do not change expected pixels to accommodate
a production failure. Calibration receipts belong to the round's history.

## Limits

These tests do not establish uncoloured-pattern tint correctness, nested group-plane
semantics or profile-based conversion behaviour. Those remain separate clause witnesses.
They do not prove performance neutrality and introduce no production cache changes.
