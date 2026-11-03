# 2658 — A straight segment's length is measured by correctly rounded operations alone

Session 1912, a single round (pixels, dependency). Status: **accepted** and **built** in
`crates/pdf-render/src/dash_end.rs`'s `segment_length`. Context: CI's `miri (pdf-render)` job red
since 7411cfc6 on two `dash_end` tests that pass natively. ADR 2659 is this round's other decision.

## The decision

`dash_end` decides whether §8.4.3.6's pattern places a zero-length dash exactly at an open subpath's
end by comparing `dash_phase + length` reduced by the period with a dash's position, for equality
(ADR 2570). The length was a sum of `f32::hypot` per segment. It is now a sum of
`sqrt(dx² + dy²)` taken in `f64` and narrowed to `f32`: each square of an `f32` is exact in `f64`, so
the sum, the square root and the narrowing are the only roundings, and IEEE 754 fixes each to the
nearest value. The extension's direction is normalised by the same function.

## Why

The standard library leaves `hypot`'s precision unspecified, so its last bit belongs to the platform
and not to the path. Current nightly Miri perturbs such operations on purpose: at HEAD, under Miri's
default, `each_subpath_answers_for_itself` lost the dot at the end of a vertical rule of 6 because
`hypot(0, 6)` came back a hair off 6. With `-Zmiri-deterministic-floats` the same tests passed over
eight seeds, and with the new function they pass over thirty-two seeds of the perturbation. A
comparison for equality may only consume a value every platform computes alike, so the fix is in the
measurement, not a margin (none is derived from §8.4.3.6, whose position "a hair short of the dash" is
in the gap) and not a `cfg(miri)` skip, which would hide the platform dependence Miri exposed.

## What it costs, and what it does not

Nothing on this machine: over 199 480 297 finite `f32` pairs — random bit patterns, sixty-fourths and
thousandths — the new function and glibc 2.41's `hypotf` agree to the bit, so no page here draws
differently. Where a platform's `hypotf` is less exact (the standard library permits it), the two
lanes now agree with this one instead of with that library. An axis-aligned segment measures its
extent exactly, which the test `a_segments_length_is_exact_where_it_can_be` holds.

`crates/pdf-render/src/closing.rs`'s `distance` has the same shape — `hypot` summed and compared for
equality with a dash boundary — and its tests passed sixteen Miri seeds at HEAD; it was not this
round's file, and moving it onto `segment_length` is the next round's to take.
