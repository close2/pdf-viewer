# 1685 — Platform checks share their package selection

Session 1685, 2026-10-21. Stability follow-up to review 1679 section 1; ADR 2204.

**Change.** One script selects the portable packages and all their Cargo targets for both local
verification and CI. CI retains native executable builds and runtime tests, and installs the
matrix's target standard library explicitly. The macOS selection includes the Unix file-system
core and ABI; native GTK/Qt remain Linux checks. No production Rust changes in this round.

**Boundary.** The coverage audit derives binary ownership from Cargo and shipped binaries from
CI's snapshot matrix. Missing packages, missing or wrongly assigned platform targets, unknown
binaries, feature-gated binaries and missing public ABI libraries fail loudly. Wrapper tests
observe actual arguments and warning flags, including encoded flags, and require Cargo failures
to propagate. The same selection serves printing and execution.

**Gates.** Platform audit and its six calibrated omissions passed; focused conformance test
1 passed, 5 s hold, 0.36 GiB peak. Scoped formatting, shell syntax and diff whitespace checks
passed. Full Windows check failed exit 101 at the absent MSVC archiver; retry with installed
LLVM tools failed exit 101 at missing Windows SDK assert.h. These are local prerequisites,
not successful cross-checks. macOS with LLVM failed exit 101 because the Darwin SDK is absent
(the C build reached incompatible Linux headers). SDK-independent core subsets remain queued.
Record conformance passed all 3 tests.
Full integration conformance, clippy and documentation gates remain the batch's obligation.

**Limit.** The full portable UI dependency graph reaches TLS C code; installed Rust targets and
LLVM alone do not supply native SDK headers. The script retains this coverage and fails on the
missing prerequisite rather than reducing the package list. Native CI must validate these paths.
