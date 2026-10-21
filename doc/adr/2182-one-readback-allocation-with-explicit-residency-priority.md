# 2182 — One readback allocation with explicit residency priority

Status: **rejected after measurement**. Session 1673, 2026-10-21.
Context: ADRs 2030 and 2150. Storage retention was removed; existing pipeline memoization remains.

## Measured decision

Three interleaved process pairs on RADV 890M gave warm readback medians (nine frames per process)
of 0.49, 0.42, 0.43 ms for HEAD and 0.37, 0.32, 0.49 ms for the candidate. Warm total-frame
medians were 7.00, 8.70, 7.34 ms versus 7.58, 7.42, 9.27 ms. Two readback samples improved,
one regressed, and total-frame time did not improve consistently. This experiment does not
justify the additional retained owner, budget priority and frame-local diagnostic state.
It does not establish that allocation reuse cannot help another workload or adapter.
The subsequent turn gate observed 76–94% background GPU activity and refused judgment against
its 20% ceiling; the instrument runs did not record that counter. Their quiet-device validity is
therefore unestablished, and these figures are not an accepted performance comparison. Removing
the optional prototype is the conservative response to insufficient evidence of a benefit.

All twelve frame/zoom instrument runs exited 0 and all paired zoom digests agreed. The frozen
candidate also contained ADR 2188's line selection; its stage-specific result is recorded there.
Cargo's later launch invocation triggered another feature-graph build, so that redundant build
was stopped after the successful instruments. A separate prebuilt hold passed the UI launch
gate but failed GTK/Qt bands and encountered the busy-device refusal in the turn gate; it was
stopped rather than collecting more unjudgeable timings. Final integration checks
the cache-free source. The measured prototype, tests, hashes and contextual patch are preserved
under `/home/AI/r1673-rejected-cache/`, outside the worktree that batch close removes; the scratch
copy and earlier frozen performance export remain intact during integration.

The following sections record the rejected prototype's design and verification, not shipped APIs.

## Boundary and cost

A backdrop readback creates a storage buffer, a parameter uniform and their bind group for
every frame. Its mapped result must remain independent: pending copies can outlive later frames.
The fresh baseline measured warm readback at 0.37–0.70 ms on RADV 890M, lower than the handover's
0.8–0.9 ms premise. That is the whole readback span, not an attribution of that span to allocation.
The prototype is retained only if the paired measurement justifies its additional state.

The candidate keeps one exact-byte-size storage/uniform/group owner. Every copy rewrites all
parameters, including dimensions and pitch; equal allocation sizes need not mean equal extents.
The copy submits before another call can write its uniform. Wgpu queues those writes before the
next submitted commands, so an earlier frame sees its own parameters. The MAP_READ result and
submission index are always per copy, never cached. Replacing the cache cannot invalidate a
submitted copy, whose command buffer retains its own handles.

## Admission and priority

Charge storage bytes plus the 32-byte uniform to the shared residency budget. Replacement drops
the old owner before charging the new one. Denied retention uses the same per-frame allocation
and drawing path and returns a typed, frame-local ReadbackNotRetained report with all three
budget numbers. Pipeline compilation and its possible refusal remain separately memoized.
Ordinary non-backdrop frames release optional readback storage.

Retained geometry has priority over this optional storage. Before first stream admission, release
the cache only if doing so allows the streams to fit. Whole-walk preflight discounts reclaimable
cache bytes but does not release them speculatively; a no-op or failed build keeps the cache.
Otherwise a scratch allocation could cause a sticky whole-walk refusal or force later zoom steps
to upload geometry. The stream-byte sum has one implementation shared by these admission sites.

The budget counts logical retained ownership. In-flight submissions may still retain old GPU
allocations after their cache owner refunds its charge. This change does not introduce an
aggregate in-flight memory limit and makes no such claim.

## Verification

Private direct-copy tests cover reversed collection of pending copies, dropping one pending copy,
equal-byte extents with different pitches, size replacement and refunds, and a denied-retention
snapshot surviving a later successful admission. Five tests pass with exact integer pixel values.
Public tests cover retained-stream priority, zero-upload replay, recovery after dropping streams,
a useful whole walk that fits only after reclamation, and a no-op whole walk preserving storage.
Three public tests pass. Isolated negative controls fail at the intended assertions: removing
preflight reclamation declines a useful whole build, while disabling admission priority leaves
zero retained stream bytes instead of 3840. Both return exit 101; shared source is untouched.
These tests establish the prototype's ownership behavior, not a performance benefit. Its tests
were preserved with the rejected prototype rather than left as tests of an absent API.
