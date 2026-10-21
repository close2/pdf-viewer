# 1677 — The documentation gate follows the workspace, and the comment sweep reads C++

Docs round, batch one hundred and four, 2026-10-21. ADRs 2190, 2191; no question needed.
**Population.** Cargo metadata names 44 workspace crates; 88 doc-enabled targets map to 84 output names;
9 crates were held, 35 outside. The baseline had 966 located diagnostics in 28 crates.
Only 8 of the 35 outside crates were initially clean, disproving the brief's "most" premise.
`pdf-model`, already held, also had two stale links; compilation and clippy did not see them.
**Built.** The kept instrument made 849 fixes: 564 editorial brackets, 285 private links.
117 cases needed a reading: moved paths, function/module ambiguity, stale names, and
private, test-only, feature-gated, nested or enum-field names that need code spans.
Four further CLI cases appeared after its library built, three without source locations;
the sentence found them. The instrument's success cannot replace the full build's exit.
The packet-remedy comment now names the actual failed-requirement filter instead of
claiming PacketFault's identifier method knows the target. No production code changes.
The gate now holds all 44 crates with --workspace; all 84 generated indexes exist.
Cargo -j2 stays in the 12 GiB large lane; new workspace members enter automatically.
**C++.** Both comment-history gates now include .cpp and assert the population was visited.
The numbered-round reader skips raw strings, including prefixes and multiline contents.
The ordinal gate first failed on two Qt comments; cleanup retained the Table 233 quotation
and ADR 0248 while removing chronology. The focused nine tests then passed.
**Proof.** The isolated Rust comment patch has 207 files and 1918 added/removed lines;
all are comments and non-comment multisets equal the stable pre-edit snapshots.
The Qt patch has 12 added/removed comment lines and the non-comment multiset equals HEAD.
The two conformance readers intentionally change their source population and have fixtures.
**Gates.** Baseline rustdoc exit 101, 51 s/4.27 GiB. First rebuilding attempt hit the
12 GiB wrapper ceiling at 12.63 GiB/43 s, exit 137. Same lane, Cargo -j2 retry: 71 s/2.20 GiB.
Whole-workspace rustdoc passed: 49.8 s/2.26 GiB (3816 s including queue).
Actual state docs exit 0: 55.5 s/2.74 GiB hold, 309 s including queue; 84 indexes generated.
Bare fmt exit 0/11 s; conformance exit 0/196 s; quotations exit 0/228 s, reporting
49 live/16 record/3 ledger divergences. CPP focused tests 0/3 s, clippy 0/1 s, rustfmt 0.
The three packet-filter comment lines postdate those bare checks; integration rechecks them.
**Limits.** Native default features only; feature-gated workers and foreign host configurations
need their own builds. GPU files were excluded from edits, included in the gate.
Generated Qt C++ maybe-uninitialized warnings remain outside the rustdoc warning gate.
Full paths: scratchpad/r1677/files.txt; isolated proof: comments.patch and comment-proof.log.
**Proposed habit.** Rebuild all documentable targets after libraries pass; keep checking the
whole build's exit because later binary diagnostics can lack locations the instrument needs.
