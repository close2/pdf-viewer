# 1679 — Stability depends on ownership and cache dependencies

Session 1679, 2026-10-21. Owner-requested stability review alongside parallel rounds.
Record: `doc/reviews/1679-stability-at-state-boundaries.md`; no production edits in this round.

**Finding.** File size alone does not explain the regressions. The confirmed failures cross
specific boundaries: Unix transport compiled on Windows, Linux confinement asserted on macOS,
a black-generation pair omitted from image conversion, a converted solid colour reused after
its inputs changed, and per-font proof followed by writes keyed to a shared descriptor.
The archive round's new shared-descriptor test fails before descriptor isolation; the solid
operator-order example independently reproduces the cache defect on baseline and candidate.

**Design.** Keep platform capabilities explicit; pilot one typed archive prepared payload;
make proof scope and write ownership agree; preserve typed second-verdict findings; declare
all dependencies of derived paint and keep accumulated diagnostics outside saved graphics state.
One rewrite may incur several independent losses, so one permission cannot stand for all of them.
The proposed tests cross preparation/application/validation and selection/state-change/paint.

**Evidence limits.** The asynchronous-answer lifetime concern remains a hypothesis until a
reachable stale-answer sequence is demonstrated. A census of fifteen batch commits measures
shared-file exposure, not regression causation. Transient compile failures during shared edits
are distinguished from failures in frozen exports. No general framework or mass rewrite proposed.

**Parallel work.** Independent owners implement in parallel; cross-cutting edits get one writer.
Frozen exports identify what was measured. Separate topic commits/worktrees are recommended for
future review and bisection, without changing this batch's prescribed commit procedure.

**Gates.** Read-only git churn census exit 0. Production tests are attributed to their implementing
rounds, not claimed for this review. The batch conformance walk reached the record/frontier checks
with no failure there; its sole failure was a sibling's teaching path, since corrected.
Focused record/teaching checks exit 0 (6 passed); integration retains full workspace and corpus gates.
