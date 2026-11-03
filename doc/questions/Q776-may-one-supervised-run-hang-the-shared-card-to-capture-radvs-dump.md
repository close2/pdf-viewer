# Q776 — May one supervised run of the clip test on the RX 580 capture RADV's hang dump?

Source: round 1916 (single round), ADR 2665.

## The question

On 2026-11-03 `render-raster`'s `clip_that_cuts_nothing::a_clip_that_contains_a_mark_takes_nothing_from_it`
hung the shared RX 580 at its 2× frame (14:02:33, the kernel names it), inside a parallel suite. Round 1916
found no fault on our side: every loop the test's shaders reach is bounded, and on lavapipe the Khronos
validation layer's core, synchronisation and GPU-assisted checks report no error of ours. What would
show whether the stuck stream is RADV's is `RADV_DEBUG=hang`, which writes the hung command stream and
the shaders to `~/radv_dumps_*` — and it is taken only on the card, by running the test there.

The round's conduct rule forbids a run suspected to hang without a fix, and a hang resets the card,
which cancels the other user's work. So the run is the owner's to allow, or not:

1. **Allowed, once, supervised**: the test alone, `--exact`, with `RADV_DEBUG=hang` and the guard of
   ADR 2665 §3 installed, at a time the other user's work can lose its contexts (or after asking
   them), followed by `journalctl -k`. If it does not hang alone, that is itself the finding that the
   trigger is concurrency (ADR 2665 §1's last reading), and the next step is a parallel run of only
   that file on lavapipe-and-card pairs, which would be asked separately.
2. **Not allowed**: the hang stays unexplained, the guard and the pinning stand, and device suites on
   this card run one test process at a time.

## What is built, and what is not

Built: the pinning (a lavapipe run brings no device up on the card), the adapter log, and the hang
marker that stops every later hardware section after a hold past nine seconds (ADR 2665 §2–3).
Not built: anything that would explain the hang; the upstream report is drafted in ADR 2665 §4 and
not filed.
