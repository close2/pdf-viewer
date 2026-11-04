# 1920 — The RX 580's turn bands take three runs at low load beside round 1917's three

Session 1920, a single round run by the orchestrator (measure), no round agent. ADRs, traps and
habits: none new. Worked in the rounds worktree.

**Why.** Round 1918's turn gate read three clocks under their floors (`issue12841_reduced.pdf` turn
100.26 and seventh 99.07 against 104.2 and 102.9, `22060_A1_01_Plans.pdf` turn 146.01 against 147.9).
Round 1917 took those floors at a one-minute load of 7.7 – 47.3 from the other account, so the
smallest of its runs was already a loaded one and ADR 1969's floor sat above a quiet machine's turn.

**Taken.** `tools/bands.sh retake --again --runs 3 --gates turn --round 1920` on RADV at a load of 4.4
– 13.3, with the warm build stopped so no compile ran beside it; exit 1 by design, since `retake`
writes only when every gate ran. Its three turn logs were copied as `turn-4.log` … `turn-6.log`
beside round 1917's eight launch, three toolkit and three turn logs and written with `tools/bands.sh
write <that directory>`. Only `turn-path.toml` moved: launch and toolkit wrote back as they were.
`issue12841_reduced.pdf`'s turn band is now 78.1 .. 152.4 ms, its quickest run 92 ms against 1917's
123: that row's turn moves with the other account's load by a third, and its band says so.

**Gates.** Turn on RADV exit 0, 33 figures banded, 33 judged, 0 outside, at a load of 4.8. No amdgpu
reset (`journalctl -k | grep -ci 'gpu reset'`: 0). `cargo test -p conformance`: exit 0.

**Unfinished.** `bands.sh retake --gates` still cannot write a part of a machine's bands; the merge
by hand is the second here. A load-sensitive row such as `issue12841_reduced.pdf` widens its band
by every loaded run it is given, which is ADR 1969's rule doing as written, not a decision about it.
