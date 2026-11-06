# The machine-local places the tools use, defined once (ADR 2656).
#
# Sourced, never run: `. "$(dirname "${BASH_SOURCE[0]}")/places.sh"` near the top of a script, and
# `bash -c '. tools/places.sh; printf %s "$NAME"'` from a Rust test that needs one of them, so that
# no script, test or document carries a machine's home directory. Every name below is the
# environment's where the caller set it, and otherwise a default derived from `$HOME` and never from
# a user name: the two roots are the whole of what a new machine decides, and every other place is a
# name under one of them.
#
#   PDF_VIEWER_SCRATCH  $HOME/scratch           the large, temporary disk: build directories
#                                               (PDF_VIEWER_BUILDS) and worktrees
#   PDF_VIEWER_AGENT    $PDF_VIEWER_SCRATCH/agent   the orchestration's own files: the heavy-walk
#                                               lock and its log, the gates log, the briefs, the
#                                               arms, and the data no tree may carry
#
# Assigned with `:=` and not exported: a script that runs another runs one that sources this file
# too, so both read the same answer, and a caller's override reaches both through the environment.

: "${HOME:?the machine-local places are derived from HOME, and HOME is unset}"

: "${PDF_VIEWER_SCRATCH:=$HOME/scratch}"
: "${PDF_VIEWER_AGENT:=$PDF_VIEWER_SCRATCH/agent}"

# The heavy-walk lock every round's walks queue on, and the log of what each hold paid
# (`tools/bounded.sh --lock`, ADRs 1646, 1662).
: "${HEAVY_WALK_LOCK:=$PDF_VIEWER_AGENT/heavy-walk.lock}"
: "${HEAVY_WALK_LOG:=$PDF_VIEWER_AGENT/heavy-walk.log}"
# The replays the lock's cost was argued from (ADR 1756 section 5, ADR 1790 section 6).
: "${HEAVY_WALK_REPLAY:=$PDF_VIEWER_AGENT/lock-replay-1485}"

# The root every build directory but the main checkout's is made under: a worktree's
# (`tools/worktree.sh`), the batch's and each export's.
: "${PDF_VIEWER_BUILDS:=$PDF_VIEWER_SCRATCH/cargo-target}"

# The batch: its worktree, its build directory, the root each export's build directory is made
# under, the root of each batch's `arms-<first>/`, and the merge's gate log (`tools/batch.sh`).
: "${BATCH_WORKTREE:=$PDF_VIEWER_SCRATCH/worktrees/pdf-viewer-rounds}"
: "${BATCH_TARGET_DIR:=$PDF_VIEWER_BUILDS/pdf-viewer-batch}"
: "${BATCH_EXPORT_TARGET_ROOT:=$PDF_VIEWER_BUILDS}"
: "${BATCH_ARMS_ROOT:=$PDF_VIEWER_AGENT}"
: "${BATCH_GATES_LOG:=$PDF_VIEWER_AGENT/batch-gates.log}"

# Where a batch's brief is, as `batch-<first>-brief.md` (`tools/round.sh`; `ROUND_BRIEF` names one
# file instead).
: "${ROUND_BRIEFS:=$PDF_VIEWER_AGENT}"

# A saved .NET SDK, as `dotnet/dotnet`, and its NuGet packages, as `cs/packages`
# (`tools/dotnet-check.sh`; `DOTNET_ROOT` and `NUGET_PACKAGES` still win).
: "${DOTNET_CHECK_SAVED:=$PDF_VIEWER_AGENT/dotnet-check}"

# ITU-T T.88 Annex K's conformance data, which its licence keeps out of the tree
# (`crates/pdf-sandbox/tests/t88_conformance.rs`, `doc/third-party-data.md`).
: "${T88_CONFORMANCE_DATA:=$PDF_VIEWER_AGENT/specs/T.88/Software/conf/JBIG2_ConformanceData-A20180829}"

# The Ghent PDF Output Suite 5.0, which its licence keeps out of the tree: `tools/fetch-gwg.sh`
# unpacks it here, pinned by hash, the way the other corpora under doc/corpora/ are submodules
# (`doc/gwg-output-suite-provenance.md`, ADR 2728). The default is the tree's own doc/corpora/,
# git-ignored, so it is derived from this file's place rather than from `$HOME`.
: "${PDF_VIEWER_GWG:=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/doc/corpora/gwg-output-suite}"

# The lock the shared hardware graphics adapter is serialised on, taken by the code around each
# device section and never by a shell around a build, a gate or a suite
# (`tools/conformance/test-support/gpu_lock.rs`, ADR 2664). The default is the system's own file
# where it exists — the one `with-gpu-lock` takes on the server VM — and empty where it does not,
# which means no lock: a machine whose adapter is its own waits for nobody. Not derived from `$HOME`
# because it is not this account's place but the machine's, shared with every account on it.
if [ -z "${PDF_VIEWER_GPU_LOCK+set}" ]; then
    if [ -e /run/lock/gpu.lock ]; then PDF_VIEWER_GPU_LOCK=/run/lock/gpu.lock; else PDF_VIEWER_GPU_LOCK=; fi
fi
