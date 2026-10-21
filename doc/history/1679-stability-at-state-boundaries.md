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
Frozen exports identify what was measured. The owner requested regular integration commits;
three checkpoints now preserve reviewed work, and the canonical batch procedure requires them.

**Gates.** Read-only git churn census exit 0. Final batch integration: 39/39 gates passed,
2795 s gate wall time; merge-readiness check exit 0. Workspace nextest: 7517 passed, 53 skipped.
Formatting, warnings-denied clippy, conformance, stable/nightly fuzz checks and all-workspace
rustdoc passed. Six final corpus arms passed (968 pages each at 1x; 965 each at 4x), with no
baseline error increase; all digests/errors/exclusions match the pre-deferral candidate.
Quiet paired measurements accepted resident-line selection; readback caching was rejected and
record-replay reuse deferred because their benefit did not justify added state. GTK/Qt's explicit
judged rerun held all 14 figures within bands. Production evidence belongs to implementing rounds.
