# ADR 2224 — Toolkit input receipts name the boundary they observe

Status: **accepted** at the merge that took it, `9cd2c665` (line added by ADR 2260).

A missing core Zoom command does not establish a missing toolkit wheel callback. The callback
may carry different modifiers, the host may be busy, or the wheel accumulator may retain a
fractional step. Observe these boundaries separately before changing input handling.

`PDFVIEWER_ZOOM_PROBE` enables versioned `zoom-record` JSON lines alongside the existing human
trace. Each toolkit callback gets a process-local window identity and increasing sequence.
Entry, branch and outcome records share that sequence; uniqueness is the pair of record kind
and sequence. A window identity is not an X11 window identifier. A driver supplies its own
trial identity and associates only its owned process and window.

GTK records raw motion before borrowing its host, then records host absence, a busy borrow,
measuring-mode suppression or accepted handling. Qt records motion before its signal and the
busy or accepted host outcome. Qt distinguishes a drag from an ordinary move. An accepted
Qt motion means the host handler returned; measuring mode may still suppress the core command.
The separate core trace establishes dispatch. None of these records establishes frame acceptance.

Qt records every wheel entry before modifier branching, including pixel and angle deltas,
modifiers, phase and source. A branch record distinguishes zoom, scroll and toolkit delegation.
Busy rejection remains visible. The diagnostic does not change event acceptance, suppression,
wheel accumulation or commands. Delegation must not acquire an invented host outcome.

A synchronous C++ zoom-dispatch span brackets the existing Rust call. Exactly one span-local
Rust accumulator receipt reports the received amount, unit, anchor and signed step count.
It carries the process identity but no invented window, callback sequence or C++ clock value.
A consumer may associate it only inside a complete, unnested span of that process; another
window's nested dispatch makes that association ambiguous. This avoids extending the bridge
ABI merely for diagnostic correlation. Zero steps is legitimate accumulator behavior.

A fresh-motion instrument captures a complete byte boundary before each requested move and
requires new matching entry and outcome records. Reusing the last match in the whole log can
turn a stale pointer into a fictitious calibration. Truncation, reset, duplicate receipts,
suppression, ambiguous callbacks and a degenerate coordinate mapping must fail explicitly.
Freshness alone does not prove that a physical stimulus caused a callback. Focus, ownership,
final anchor and backend checks remain independent obligations of the driver.
