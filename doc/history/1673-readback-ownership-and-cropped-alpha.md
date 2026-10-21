# 1673 — Readback ownership and cropped alpha

Session 1673, 2026-10-21. Pixel round; ADRs 2182 and 2183.

**Correctness.** Cropped medium-less output now uses the same nearest-integer straight-alpha
division as whole-page output. The eight-by-eight translucent witness agrees byte for byte on
the surviving pixels and keeps the outside transparent. The test-only baseline export fails
at the intended one-component rounding difference; the corrected output was also inspected.
The clause constrains cropping's preservation, not a particular integer rounding algorithm.

**Measurement premise.** A fresh RADV 890M baseline put warm readback at 0.37–0.70 ms,
below the handover's 0.8–0.9 ms. These are whole readback spans, not allocation attribution.
The proposed exact-size storage/uniform/bind-group cache was rejected after paired measurement.
The prototype kept each pending mapped copy independent and wrote parameters per submission.
It charged logical retained ownership, without claiming bounded in-flight physical allocation.

**Prototype boundary.** Optional readback storage yielded to retained geometry for admission.
Preflight discounted reclaimable storage; actual admission evicted it. Denial used per-frame
allocation and reported refusal. These prototype paths and tests were removed with the cache.

**Gates.** Alpha witness passes; its baseline negative control exits 101. Five private
copy/cache tests and three public residency tests pass. Two isolated priority negative controls
exit 101 at their intended assertions. Scoped GPU/render clippy exits 0 before the later replay
integration. The fresh baseline profile exits 0 (217 s held, 2.44 GiB peak).

**Decision.** Three paired warm readback medians were 0.49/0.42/0.43 ms versus 0.37/0.32/0.49 ms;
total-frame medians did not improve consistently. The small inconsistent benefit does not justify
the extra state. Cache code/tests were removed and preserved in a hashed scratch prototype;
the alpha correction remains; selection/replay has its own performance decision. All twelve
instrument runs exited 0 with identical paired zoom digests, but later gates observed 76–94%
GPU activity: quiet-device validity is unestablished. No accepted speed claim follows.

**Before replay deferral.** Cache-free GPU/render clippy exits 0; 868 tests pass, 5 skip (22.817 s tests,
126 s hold, 3.33 GiB). Combined six arms pass: 968 pages each at 1x, 965 each at 4x,
1810 s hold/3.66 GiB; no added/removed pages or error increases. The only differences from
font-only arms are bug1721218_reduced and issue16742; isolated controls attribute both to
alpha rounding, not BG propagation. Report: /home/AI/report-final-arms-1679.md.
Paired launch/turn performance acceptance remains separate; no speed claim or GPU-loop job.
After replay deferral, all six arms pass again (1549 s, 3.76 GiB), with every digest, error
and exclusion identical to the earlier candidate: /home/AI/arms-1679-selection-final/.
