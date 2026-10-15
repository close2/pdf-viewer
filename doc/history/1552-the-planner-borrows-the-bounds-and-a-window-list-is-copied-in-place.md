# 1552 — The planner borrows the bounds, and a window's list is copied in place

Slot 1 of batch eighty-eight, 2026-10-15, a pixels round. ADRs 1940, 1941; no Q; no row moves.
**Premise (a), held, and one more**: at the witness's massif peak `MarkBounds`' and the planner's
88 168 100-byte vectors live at once, and the bands' keys are 62 571 264 bytes beside them. Not one
of the 4 408 405 commands has an extent other than its bound. **Premise (b), not in its mechanism**:
the cull's walk is 7–12 ms; a window of 83 % pays for copying the 4 333 950 commands it keeps
(77–158 ms on one thread, 81–98 filtered on the pool), letting them go (31–34 ms) and the planner
measuring each kept bound twice. **Hypotheses**: (a) held — 84.1 MiB and no time, and 31 MiB more;
(b) did not, and could not: a culled list of 98 % of the commands draws as long as the page.

**Built.** The planner borrows the bounds where they were measured under the target's transform and
states only the exceptions, past one in four every extent; band keys are 32 bits; a moved bound is
measured once, not twice (ADR 1940). A whole page's cull is answered by the bounds' innermost edges
after the first ask; the kept commands are copied by an indexed iterator in place (43–49 ms); a list
of 65 536 commands or more is let go of on the pool (18–21 ms) (ADR 1941).

**Figures.** Massif useful peak 1 381.6 → 1 266.5 MiB (1 515.9 → 1 400.8 with the allocator's own),
the frame's SHA-256 the same. The crossing (third hold, ADR 1941 §3): a window of 63 % ties the page
on four threads, is 13–66 ms behind on twenty-four; one of 49 % is 97–213 and 26–85 ms ahead; one of
83 % is 61–181 / 22–61 ms quicker than HEAD and 61–202 / 69–131 ms behind its page; the half stands.

**Proof that nothing moved.** The two processor arms against `/home/AI/arms-1552/`: 968 and 965
pages, 0 digests and 0 mean errors differ (run twice, before and after the indexed copy). Every whole
and window drawing of the example has HEAD's digest at each of eight magnifications on four and
twenty-four threads. `raster_golden`'s division test: 176 pages move at some division, ≤ 1 308 px,
≤ 3 levels, ADR 1758's figures. Plants: an extremes test loosened by 3 px, a NaN bound admitted, every
extent taken as its bound, and every run stated in full each failed a test; one within the margin did
not, rightly. The tree's three `pdf-render` files differ from the measured export in comments only.

**Files.** `pdf-render/src/{reach,strips,display_list}.rs`, `viewer-host/examples/window_against_whole.rs`,
ADRs 1940–1941, `doc/{state-of-play,performance}.md`, `doc/todo/{30,69}`, this record. **Unfinished**
(ADR 1941 §4): a culled list that copies nothing, and the bound stated in commands kept.

**Gates.** `rustfmt --check` 0; `clippy -D warnings --all-targets -p pdf-render -p render-cpu -p
viewer-host` 0; `nextest` 753 passed, 0 failed; `cargo test -p conformance` 436 passed; `state.sh
docs` 0. `t2-raster_golden` 0, `t2-drawn_while_read` 0, `cpu-1x` 0, `cpu-4x` 0 (twice). Clock holds:
the first (382 s, load 4.9–8.2) read `t2-turn_path` exit 101, `issue19802` step 4.68 ms against
1.1–1.8; the second (240 s, load 1.4–3.7) read that page's child 1.34–1.42 ms on HEAD and this alike
and `t2-turn_path` exit 0, 33 of 33 inside; the third (157 s) the example. Duration 11 300 s.
