# 1678 — The session wrappers are grouped by the command they serve

Session 1678, slot 8 of batch 104, a code-quality round; 2026-10-21. ADR 2192; ADR 2193 unused.
No owner's decision was needed. Ledger edits add code paths only; no status moved.

**Choice.** The census's Session implementation had 97 methods in 1291 lines. Larger ranked
blocks belonged to active agents. The C session's document, navigation, frame, print, editing,
policy, script, report and archive wrappers now have nine modules. The root retains construction,
query dispatch, state, free helpers and its tests. No visibility or runtime behavior changed.

**Proof.** A saved original and the final files have identical token sequences for all 97 method
bodies, including literal contents; state, free helpers and tests are identical too. Only imports,
module paths and one moved rustdoc link differ. The Session block leaves the census's top five.
The reverse dependency graph contains only viewer-ffi, and the six-arm render/worker roots do not
depend on it: no pixel arm is owed by this refactor.

**Gates.** The token proof exits 0; viewer-ffi formatting exits 0. Combined viewer-ffi/pdf-sandbox
nextest passes 124 of 124 tests, including both C programs, after the split; the expanded host
suite then passes 491 tests, two existing corpus/timing tests ignored. Shared clippy exits 0.
Conformance exits 101 first at the concurrent 1675 record's gate spelling, and a later run reached
the moved teaching diagram's short path. Both have been corrected; final batch conformance
remains an integration gate, not a waived check.

**Documentation.** todo 74, the crate map, the selection/copy teaching paths and its diagram now
name the split. An existing Self::preferences rustdoc link in opening() was reported to the docs
round rather than mixed into a behavior-only change. Shared files are those paths and the seven
ledger code-path additions; the host round separately owns Window and ABI behavior.

**Proposed habit.** For a structural split, preserve a token proof that includes literals and free
helpers, then run the existing boundary tests. A line-count drop is not evidence of preservation.
