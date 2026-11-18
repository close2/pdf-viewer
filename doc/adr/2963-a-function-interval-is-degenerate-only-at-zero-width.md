# 2963 — A function's interval is degenerate only at zero width, and −0 equals +0

Session 2072. Status: **implemented.** Context: review 2038 findings B09 and B10, ISO 32000-2 §7.10.1, §7.10.2,
§7.10.4.

## 1. The findings

`function.rs::interpolate` answered `to_low` for any input interval narrower than `f32::EPSILON`, so a Type 0
function with `/Domain [0 1e-8]` returned its first sample everywhere. §7.10.1's interpolation formula states no
tolerance. Separately, the stitching function's degenerate first interval compared `Bounds 0` with `Domain 0` by
`total_cmp`, which orders −0 below +0, so `/Domain [-0 1] /Bounds [0]` was not "equal" in §7.10.4's sense.

## 2. The decision

An interval is degenerate when its width is exactly zero, and only then answers `to_low`, which is §7.10.4's own
answer for its degenerate case ("x′ shall be defined to be Encode 0"); the formula is undefined there and nowhere
else. The stitching comparison is IEEE `==`, under which −0 and +0 are one number, and carries a
`clippy::float_cmp` expectation saying the exactness is the clause's.
