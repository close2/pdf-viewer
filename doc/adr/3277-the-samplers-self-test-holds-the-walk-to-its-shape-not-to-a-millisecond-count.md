# 3277 — The sampler's self-test holds the walk to its shape, not to a count of milliseconds

Session 2144. Status: accepted. Context: ADR 0807 (the sampler), ADR 3114 (`batch.rs`), ADR 3134
(`wait.sh`), `tools/bounded.sh --self-test`, `tools/conformance/tests/bounded.rs`.

## 1. The failure

At a merge under load, `the_bounded_wrappers_self_test_holds` failed its first case: "flat tree: one
sample cost 1195 ms, which is not a fraction of the interval it has to fit". Run alone it passed.
The case already measured processor time rather than wall time, and processor time still moves
with load: on this machine the same walk of a hundred thousand rows costs 350 ms idle and from
500 to 1300 ms beside other work (shared caches, clock frequency). A fixed bound of 1000 ms
asserted the machine, not the walk.

## 2. The decision

What the case exists to catch is two defects, and each is a ratio:

- **A walk that grows faster than the table**: the quadratic walk this sampler replaced, minutes
  here. A linear walk of the table costs about ten times a walk of a tenth of it; a quadratic one
  about a hundred. The case fails above thirty.
- **A per-row cost the walk does not need**: the concatenated pid string, a second and a half by
  itself. The indexed walk costs about six times one plain `awk` read of the same rows; the case
  fails above twenty.

Each of the three measurements runs three times, back to back and interleaved, and the least of
each is compared, so that a load that rises between two runs decides nothing. The absolute
figures are still printed.

## 3. Evidence

- Under load (six busy loops and the case pinned to the same two processors), the old case failed
  with "one sample cost 1327 ms"; the new one passed (519 ms, against 53 ms for a tenth and 73 ms
  for one read).
- Calibrated (trap 13): a walk planted with the concatenated string fails the first ratio
  (8292 ms against 86 ms); a walk planted with two hundred additions per row fails the second
  (2376 ms against 65 ms).

## 4. The cost

A walk that is linear with a large constant per row below twenty reads passes, where the old bound
caught it only on an idle machine. A walk whose cost is high only on this machine, at every size,
is no longer caught here; the sampler's own deadline (`sample_deadline`) and its missed-sample
count are what bound it at run time.
