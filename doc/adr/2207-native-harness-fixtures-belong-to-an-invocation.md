# 2207 — Native harness fixtures belong to an invocation

Status: accepted; scoped validation passed. Session 1686.
Context: ADR 2206; repeated C clients and the KIO plugin driver.

Fixed C executable and writable PDF names under a shared Cargo profile let two
invocations of the same test overwrite each other. A checkout hash distinguished
KIO build caches across checkouts but did not distinguish runs within one checkout.
A successful run could consequently consume another invocation's scratch state.

Each native test now owns a temporary directory until its child processes finish.
Its compiled C client, generated forms, writable PDFs and reader directory live
there. The KIO invocation also owns its CMake cache, plugin placement and driver.
Cleanup follows the owning temporary directory's lifetime. Source fixtures stay
read-only, and Cargo artifacts remain shared under Cargo's ordinary build lock.

This separates mutable fixtures without introducing a global lock over unrelated
tests or changing the production libraries. Private KIO trees give up a warm CMake
cache; retain such a cache only if later evidence justifies a lock covering every
write and use. Optional KDE prerequisites are still reported explicitly.

Subprocess controls require different writable paths even when names match and
verify cleanup after both children finish. Real C and KIO checks retain their
existing semantic assertions. This is not a claim that arbitrary source revisions
may mutate a shared Cargo output concurrently with execution: source stability and
the round scheduler remain prerequisites of the proof.

Validation: two subprocesses used the actual metadata/profile/fixture constructor,
obtained distinct writable paths with identical filenames, and verified cleanup.
The real C clients and KIO plugin also passed in the gates profile; KIO exercised
its private CMake tree rather than taking the optional-prerequisite skip. The
combined client chain exited 0 after a 639 s hold, peaking at 1.40 GiB.
