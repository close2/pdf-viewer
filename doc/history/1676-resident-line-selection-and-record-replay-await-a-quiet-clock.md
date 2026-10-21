# 1676 — Resident line selection stays; record replay is deferred

Session 1676, slot 6 of batch one hundred and four; pixels round, 2026-10-21. ADRs 2188, 2189.
Selection accepted: final shipping launch, turn and toolkit gates passed.

**Built.** Conservative bounds over 256 consecutive resident line instances select draw ranges
without rewriting instances or changing painter order. Shared metadata is charged before allocation;
optional bounds refusal preserves whole-run drawing and reports the fallback. Omitted-instance and
refused-bounds counters leave the kept-line count unchanged. No shader arithmetic changed.

**Population.** The census requested 968 first pages: 957 drew, 11 refused, 850 record-replayed,
and 61 replayed pages contained 17,834 lines. Ten refusals need passwords; one exceeds the adapter's
side limit. Refused pages remain uncensused. The positive-tested `replay_line_census` example stays.

**Measured.** A cache-free, exclusive nice-0 hold completed three interleaved pairs, exit 0 after
235 seconds, with identical digests. GPU activity means were 3.0–18.85%; CPU load 1.04–4.56.
At 3× selection content passes changed 29.5/29.8/30.4 → 26.3/26.2/26.7 ms; at 3.6×,
21.7/20.8/22.0 → 17.6/18.2/17.8 ms. Retained whole frames improved in all six pairs; idle
construction cost another 13.4/5.0/7.0 ms. ADR 2188 preserves every whole-frame/first-frame figure.
Two earlier attempts stopped at 20.3% and 20.15% activity; rejected samples stay rejected. The
successful attempt used the same fixed ten-second rest before every process, never after-failure rescue.

**Deferred.** Replay reuse saved 983,040 line bytes per changed view, but showed no consistent frame
speedup: final pairs include 30.8 → 52.2 and 31.5 → 51.2 ms; an earlier valid prefix also worsened.
The extra reuse, replay idle eligibility and three replay-only tests were removed. Selection stays.
Exact patch, tests, complete exports and all attempts survive at `/home/AI/r1676-deferred-replay/`.
No owner GPU loop was used. Todo 75 keeps the census and the separate future prediction question.

**Gates.** Final selection-only workspace fmt/clippy exit 0; nextest 7517 passed, 53 skipped.
All six final arms pass: 968 comparable pages per lane at 1×, 965 at 4×, exit 0 after 1549 seconds,
peak 3.76 GiB. Every digest/error/exclusion matches the prior combined candidate; no error increases.
Before deferral, 876/868 scoped and 7519 workspace tests passed. Post-removal 864 passed, one failed:
a Compute budget fixture depended on deferred reuse. Restoring the original CPU-only refusal fixture
preserved its assertions; final scoped fmt/clippy and all three selection witnesses pass, exit 0.
Both stateful witnesses cover CPU/Compute, including unaligned runs. The fused reference probe lives
outside production sources without an allowlist waiver. Turn/launch/toolkit exit 0 (186/361/293 s).
Direct toolkit rerun: all 14 figures explicitly judged and in band, exit 0/26 s; full output preserved.

**Proposed habit.** Test retained state against fresh rendering across operation sequences, and reject
extra state when its measured benefit does not justify it. Correct pixels do not establish a speedup.
