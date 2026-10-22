# ADR 2228 — Zoom probes correlate fresh owned callbacks

A whole-log search for the last pointer position can reuse a stale event and make a failed
calibration appear successful. A missing core Zoom command also cannot establish that Qt never
received a wheel callback: modifiers, the host busy guard and fractional accumulation intervene.
The toolkit records in ADR2224 expose those boundaries; the probe must preserve their identities.

`tools/zoom-probe-records.py` takes an immutable complete log prefix captured before each input.
New records must preserve that prefix, remain within four MiB, have complete lines and match the
externally owned session, host, process and local window. Callback entry sequences increase;
entry, branch and outcome may share one sequence, but duplicate kind/sequence pairs refuse.
Monotonic clocks are compared only within the emitting process and clock domain.

Motion calibration requires exactly one fresh move and its accepted matching outcome. Drags,
missing or orphan outcomes, suppressed input, reused sequences, nonfinite coordinates and
ambiguous populations refuse. The display driver additionally requires one fresh core Pointer
with matching coordinates: Qt's handler-return receipt alone does not prove core dispatch.
Two distinct requested XTest positions determine a local affine mapping; a third independently
requested point must validate it. Logical-to-core scale remains a separate observation.

Qt wheel callback, modifier, selected branch and host outcome are counted separately. Accepted
zoom has one synchronous C++ dispatch span enclosing one Rust accumulator receipt. Rust's
receipt does not pretend to share the C++ clock or window identifier. Nested, foreign-window,
orphan and repeated spans refuse association. Accumulator amount, unit and anchor must agree
with the callback and branch. Zero steps from a valid sub-notch input remain a valid observation.

The parser's self-test plants stale, duplicate, reset, foreign, partial, reordered, nonfinite,
modifier, suppression and span faults, plus valid fractional and offset mappings. A conformance
test requires both successful exit and the self-test's positive closing marker. The self-test
requires all nine discovered tests; an empty-discovery control must fail before that marker. These controls
validate evidence association; they cannot prove physical causality, rendering completion,
scanout, sharpness or responsiveness. Actual display rehearsal is separately recorded.
