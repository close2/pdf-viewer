# 1440 — What a merge cannot carry is a command the owner runs and a line `state.sh` prints, and a batch builds apart from the main checkout

Session 1302. Status: accepted. Amends the build-directory sentence of `tools/batch.sh`'s header,
which said a batch shares the main checkout's build directory on purpose. Context: `CLAUDE.md`
*Where knowledge lives* (a countable fact is a command); trap 50 (two trees sharing a target
directory hand each other their artefacts); ADR 0344 (a checkout names its build directory in
`.cargo/config.toml`, never an exported `CARGO_TARGET_DIR`); ADR 1313 (`tools/batch.sh`); ADR 1439.
Code: `tools/main-checkout.py`, `tools/state.sh` (`section_main_checkout`), `tools/batch.sh`
(`open_batch`), `doc/environment.md` (*After a merge*), `doc/HANDOVER.md`.

## 1. The owner-side list is one section of commands and one section of `state.sh`

Round after round ended its record with something on the owner's disk it could not touch: the fuzz
lock in the main checkout, artefacts whose defect was fixed, a corpus a campaign found stale, the
owner's answer files uncommitted. Each was a sentence in one record, so the next round did not see
it. `doc/environment.md`'s *After a merge* now states each as a command, and `tools/state.sh
main-checkout` (`tools/main-checkout.py`) reads the main checkout without writing to it and prints
which of them has anything to do: a local edit to a path the batch changes, which stops
`git merge --ff-only`; whether `fuzz/Cargo.lock` is tracked there and agrees with the root lock;
each artefact sorted into read and unread; the targets with no seeds; the answer files uncommitted.

**An artefact is read when the tree names it**, by the first eight digits of its hash, wherever the
fix is argued — an ADR, a record, the regression test's comment. That is a test the tree already
passes for the three artefacts rounds fixed (`page/timeout-6af40bfb`, ADR 1424;
`serialize/crash-3a47ca5d`, `crates/pdf-syntax/tests/serialize.rs`; `x509/crash-60910642`,
`crates/pdf-signature/src/eddsa.rs`), and it needs no list kept beside the directory. The two it
called unread, `page/timeout-409c01ae` and `page/timeout-417c1ea5`, were read here: run once each
through this tree's `page` target, built `-s none`, they finish in 67 ms and 3.5 s under the
documented `-timeout=60`, beside 189 ms for the fixed one — no longer timeouts; which change ended
them was not bisected. Naming them here is what makes the command call them read.

Today's reading, first run: one local edit in the way (`.gitignore`, the owner's `/doc/qpdf` line,
which this batch's `.gitignore` change will meet), the main checkout's fuzz lock untracked and two
versions apart from its root lock, three read artefacts removable, five targets unseeded (`shaping`,
`jbig2`, `jpx`, `xfdf`, `linearize`), fourteen answer files uncommitted. Those are its numbers on
one day; the command is the source.

## 2. A batch worktree builds in a directory of its own

`cargo test -p conformance` in the batch worktree linked a `conformance` library compiled from the
main checkout: `strings` on the test binary found `/home/cl/projects/pdf-viewer/tools/conformance`
as its `CARGO_MANIFEST_DIR`, so every sibling's run read the main checkout's ledger and
`doc/todo/65` rather than the worktree's until a touch rebuilt it. Cargo names a path package's
artefacts relative to its workspace root, the two trees share `/home/AI/cargo-target/pdf-viewer`,
and an artefact built in the main checkout after the worktree's files were written is fresh to the
worktree by modification time — trap 50's shape, met by a gate rather than a measurement.

`tools/batch.sh open` now writes the worktree's `.cargo/config.toml` naming
`/home/AI/cargo-target/pdf-viewer-batch` (`BATCH_TARGET_DIR` overrides it), and only where the
tree's `.gitignore` already ignores that file, so it can never join a batch's population. **One
directory for every batch**, not one each: the worktree's path is the same every batch, so the
second batch finds it warm, and the cold build the old sentence refused is paid once rather than
per batch.

## Costs

A second build directory's disk, and one cold build. `main-checkout.py` greps the tree once per
artefact, which is seconds while the directory holds dozens; `slow-unit` warnings are counted and not
sorted, since libFuzzer writes them for inputs it did not stop on.
