# 2240 — GTK settled captures identify the completed source

Status: accepted for diagnostic use. Session 1703, 2026-10-22.

## Decision

Observe the original Query::Frame raster before stand-in composition. A qualifying
whole-page placement must retain its pointer identity and page, match current geometry
and extent, and not be retained. Require exactly one final placement before texture
filtering; a failed texture cannot hide another page. Transitions and oversized/partial
replacements cannot masquerade as this completed source.

Opt-in zoom tracing maintains a refresh generation, clears stale observations before
composition, and publishes applied geometry only after texture placement. The subsequent
native paint carries the same document, page, generation and source index, actual widget
bounds, fixed allocation and content geometry. Observation never controls rendering.

Requested captures are caller-owned diagnostics: numeric IDs, bounded request/raster
sizes, no-overwrite claims/files and opaque interior ROI. Failed IO disables further
capture and is reported. All capture work is excluded from latency claims.

## Verification

Four library tests distinguish same-sized partial rasters, stale generations, changed
geometry, missing sources, retention and opaque interior bytes. All pass. Python parser
self-tests include positive and 22 negative settled-capture joins. The parser rejects
later refreshes, missing paint and mismatched identity/geometry. A paint receipt alone
proves neither screenshot mapping nor final compositor scanout; ADR 2241 owns that limit.
