# 1686 — Native tests own their artifacts and fixtures

Session 1686, 2026-10-22. ADR 2206 and ADR 2207.
Started 2026-10-22T02:15:14+00:00; recorded completion 2026-10-22T02:53:51.135516+00:00, 2317 s elapsed.

**Change.** Three native integration files share a test-only Cargo artifact reader.
It resolves exact package identities once, requests the invoking native profile,
and accepts only matching Cargo JSON library or executable attestations in that
profile directory. Fresh artifacts remain valid. Unexpected layouts and profile
mismatches fail. Each invocation owns its C programs, writable PDFs and KIO
CMake/plugin tree. Existing optional-prerequisite skips and semantic checks remain.
No production ABI, worker protocol or renderer changes; no new workspace crate.

**Gates.** Scoped formatting and warnings-denied all-targets Clippy passed, exit 0
(Clippy 44 s hold, 0.84 GiB). Development nextest: 99 passed, 0 skipped.
Gates-profile suites: 7, 8 and 7 passed; actual C clients and KIO plugin ran.
KIO browsed/stat/read the document and deleted then inserted a page; no skip.
The client chain exited 0 after 639 s hold, 1.40 GiB peak, 1262 s including queue.
Six isolated helper controls passed, including separate-process fixture cleanup.
Removing profile verification caused the stale/wrong-profile control to fail
with exit 101 as required; calibration driver exit 0. Shared conformance passed
in round 1687 after final helper placement; final integration accepted with launch follow-up in ADR 2209.

**Attempts and limits.** Earlier attempts stopped on initial helper placement and
sibling in-flight transform errors; final placement and scoped checks resolved them.
Native Linux clients are covered; cross-target execution needs an explicit contract.
Private fixtures do not permit concurrent source revisions to rewrite shared Cargo
outputs safely. KIO deliberately forgoes a shared warm CMake cache. No timing claim.
Evidence: scratchpad/r1686/clients.log, clippy2.log and control3-*.log;
external scope/report: /home/AI/paths-1686.txt and /home/AI/report-1686.md.
