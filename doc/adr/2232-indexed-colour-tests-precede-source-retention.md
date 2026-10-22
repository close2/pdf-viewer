# 2232 — Indexed colour tests precede source retention

Status: accepted. Session 1699, 2026-10-22.

## Decision

Pin independent Indexed/ICC paint expectations before changing the source constructor.
The existing thirteen direct-ICC tests remain unchanged; ten new controls distinguish
lookup entries, initial zero, profile and component identity, saved state, Form resource
shadowing, late intent/BPC, fill/stroke/text/stencils, alpha and group return.

Literal encoded XYZ knots and rational byte/255 interpolation feed the existing pure
matrix/transfer arithmetic. Interpolation is not quantized a second time. The one-level
pixel tolerance is unchanged. Gray neutrality and a cold-path comparison are supplemental,
not a complete independent Gray conversion oracle.

## Evidence

Checkpoint 9e426dd5 precedes the production change on fixed 9cd2c665: 14 passed,
9 failed intended assertions, with all original thirteen passing. The candidate passes
23 tests. Reordering the text and Gray assertions makes independent expectations fail
before cold comparisons, without changing the oracle. A later helper-parameter rename
is lexical only. ADRs 2234/2235 retain candidate, mutation and cost evidence.
