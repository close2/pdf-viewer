# 1451 — A batch opens by warming its build directory, and the prose sweeps answer as one section

Session 1308. Status: accepted. Context: ADR 1440 (a batch builds in a directory of its own, "one
cold build"); ADR 0344 (the target directory is named in `.cargo/config.toml`, never exported);
ADRs 1273, 1403, 1427, 1437 (the sweeps gathered in section 3), ADR 1416 (section 4). Code: `tools/batch.sh` (`warm`),
`tools/state.sh` (`section_prose`, `prose_line`), `tools/conformance/tests/state_sections.rs`,
`tools/conformance/tests/todo_citers.rs`, `doc/todo/README.md`,
`doc/environment.md` (*Build directory*).

## 1. What the batch's own build directory costs, measured

Batch forty-seven was the first to open on `/home/AI/cargo-target/pdf-viewer-batch`, so the first
build there was cold. Taken on 2026-10-01 in the batch worktree, `sccache` in front as
`doc/environment.md` says, `time` from the shell:

| build | wall | CPU (user + sys) | what it compiled |
|---|---|---|---|
| `cargo build --workspace --all-targets`, empty directory | 354 s | 1 643 + 158 s | everything |
| the same, after two siblings had edited `pdf-font` and `raster-gpu` | 553 s, one lock wait | 1 380 + 140 s | 25 workspace packages |
| `cargo nextest run --workspace --no-run`, after further edits | 180 s, one lock wait | 1 603 + 169 s | 24 |
| `cargo build --workspace --all-targets` again at once | 7 s | 19 + 4 s | 5 |

Nothing here was quiet: the load average rose from 0.3 to 46 during the cold build, because two
siblings' builds under the `release` and `gates` profiles ran beside it. `sccache`'s server is
shared by every round, so its counters say only that the whole machine made 208 Rust hits and 977
misses while the cold build ran; ADR 0344's categories, not a rate, and no attribution to one build.

**What cargo does with six rounds at once, observed rather than assumed.** A `cargo build -p
pdf-sandbox --all-targets` started 45 s after the cold build held no `rustc` child until the cold
build ended: cargo locks a profile's directory (`debug/.cargo-build-lock`), and every `dev` build
queues behind the one holding it — no work is duplicated within a profile. The `release` and
`gates` builds had 24 children each at the same moment: each profile has its own lock, so they
compile the same dependencies beside the `dev` build, three times over, which is where the load
came from.

**And what the numbers say about "one cold build".** The empty directory costs about as much as a
rebuild of the workspace's own packages: 1 643 s of CPU against 1 380-1 603 s after an edit to
`pdf-font`, which nearly every package depends on. The dependencies are the smaller share. So the
directory being "warm" is a state that lasts until a sibling edits a low crate, and ADR 1440's one
cold build per directory is true and small beside what six rounds editing shared crates rebuild
anyway.

## 2. `open` warms the directory, detached

The rounds of this batch met the cold directory within a minute of one another, each waiting on the
lock for the first six minutes of its work. `tools/batch.sh open` now starts `cargo build
--workspace --all-targets` in the new worktree, detached, logging to `scratchpad/open/build.log`,
and returns. The orchestrator writes the briefs after `open`, which takes longer than the build, so
its cost overlaps work that happens anyway; a round arriving first waits on cargo's lock exactly as
it would wait on a sibling. From the second batch on the directory persists (ADR 1440), and the
build at `open` recompiles what the merge changed — the same cost a round would otherwise pay first.

Detached rather than in the foreground, because a foreground build adds six minutes to the batch's
wall clock before the first brief is written. Only the `dev` profile: the `release` builds rounds
make are `conformance`'s sweeps and one crate's examples, and the `gates` builds are the merge's.
Skipped where the tree has no workspace manifest — the throwaway repository `tests/batch.rs` opens —
and under `BATCH_WARM=0`.

## 3. `tools/state.sh prose`

"Is the prose true" was seven commands in five places: `comments` (ADR 1403), `superlatives` (ADR
1427), the navigation section's `overtaken` and `unread`, `names` (ADR 1273), `tests/variables.rs`,
and ADR 1437's program-name sweep inside the ledger gate. `prose` runs each as its own section runs
it and prints one line per sweep — the line that is its count — and under it the command that lists
what it counted. It computes nothing: `prose_line` is a filter, as `run` is. The program-name sweep
has no count of its own, since it is one of the ledger gate's checks; its line is that gate's
verdict, and when the gate fails the problems it printed follow, so a sibling's mid-edit row is not
read as a stale program name. Seven lines rather than six, because overtaken and unread are two
counts over two populations. In `all` and `quick`; four seconds warm.

`tests/state_sections.rs` now holds the script's lists to its arms both ways: a name in `all`,
`quick` or `composed` with no `name) section_…` arm, and an arm neither `all` nor `composed` names,
each fail by name. It found nothing but its own first parse, which dropped `jpeg2000` for its digit.

## 4. A done todo file's `Cited by:` line is held to its citers

ADR 1416 keeps a done todo file whole while something cites it, and the file's header names what.
Nothing read that line against the tree. `tests/todo_citers.rs` does: in the line's first sentence,
a backticked directory must hold a Rust source mentioning `todo/NN`, a backticked file must mention
it, and where the sentence speaks of a ledger note each `§` in it names a row whose note must. A
backticked command (`doc/todo/39`'s `tools/state.sh annex-o`) is not a citer. All eight lines
resolve today; the planted test names a crate that does not cite and fails without it.

## Costs

A detached process the orchestrator did not wait for: a build that fails at `open` says so only in
its log. The table's figures are one day's under a load nobody chose, and are the ADR's, not a
baseline — `time` on the same two commands re-takes them.
