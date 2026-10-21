# 1681 — Frame estimates name their measured view

Session 1681, 2026-10-22. Owner's estimate report, todo75 §8; ADRs 2196/2197.

**Change.** A completed price now applies only to its page/ink, command population, complete target,
output extent and graphics coverage. Transition work cannot borrow or replace a page sample.
Incompatible samples use the existing seed with explicit provenance. Arrangement trial flags,
held-scene bookkeeping and refusal policy remain independent; zoom does not schedule GPU probes.
A seed can still stay fixed through a gesture. No geometric scaling is invented.

**Gates.** Six stateful regressions cover fit/zoom/pan/fit, identity/extent/coverage, transition
samples, held scenes and once-only trials. All 19 CPU policy tests passed; all six regressions
fail when compatibility is removed in an isolated export (clean negative-control wrapper exit 0).
Scoped warnings-denied clippy and viewer-device/viewer-ui tests passed: 228 passed, 2 ignored,
exit 0, 553 s hold, 2.66 GiB peak. Frozen candidate is baseline 020a86f3 plus nine owned paths.

**Hardware.** A test-only probe in that export used actual ReadBack offer, draw, completion and
measurement on AMD Radeon 890M (RADV STRIX1), Vulkan. Fit, zoom, pan and fit again each became
Measured after completion; incompatible views, coverage and page keys did not borrow the sample.
Quiet GPU means 3.7–4%, load 1.81; exit 0, 62 s hold including 60 s settling, 0.10 GiB peak.
This proves the headless built-scene path, not a live surface or kept-scene timing improvement.

**Limits.** Xvfb selected llvmpipe without DRI3; its traces are not hardware evidence. A preceding
probe refused before measurement because CPU load exceeded 12, despite quiet GPU activity.
Held turn checks passed all 33 bands. Launch failed one of 59 banded figures (13 not judged for
carve-out): opt_demo warm open 0.157 ms against 0.080–0.140 ms. That metric times document opening
without a renderer; the failure is retained, no band changed, and no blanket launch pass claimed.
Final combined corpus/integration checks remain the batch's obligation.
