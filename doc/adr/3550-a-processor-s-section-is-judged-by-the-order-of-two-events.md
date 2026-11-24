# 3550 — A processor's section is judged by the order of two events, not by a wall-clock bound

Status: **accepted**, implemented (`render-raster/tests/gpu_lock.rs`,
`a_processor_s_adapter_never_waits`). Session 2212. Follows ADR 3134 (the waiter's self-test orders
events rather than counting seconds), ADR 3114 and ADR 3277; ADR 2664 (the lock is the code's).

## Context

The test held a planted lock for 1.5 s and asserted a lavapipe bring-up and frame finished in less
than that. At a load average of 45 to 75 a software adapter's bring-up alone passes 1.5 s, and the
test failed with no lock taken.

## Decision

The planted holder holds until the parent closes its input, and the parent closes it only after
the section's child has ended. A child that ended at all ended inside the hold, so success is the
order of the two events, and the holder's still running when the child ends is asserted. A section
that asked for the lock waits for a hold never released, until `GPU_LOCK_TIMEOUT` — set to 20 s for
the child — panics naming the lock; the parent reports that as "a processor's section waited for
the planted hold". The log's `bring-up` line stays asserted absent.

## Consequences

No load makes the test fail while the hook leaves processor adapters alone. Planted (the hook made to
lock a `Cpu` adapter too), it fails in 25 s with that sentence.
