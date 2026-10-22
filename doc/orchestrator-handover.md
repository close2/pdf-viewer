# Hand-over: orchestrating the rounds

Status: **record**, written 2026-10-21 by the orchestrating session after batch one hundred and
three merged (`5ded4916`) and the owner said "please do not start a new batch". It is for whoever
runs the next batch — the owner, or a fresh session on the agent account — and it is the layer
*above* a round: what a round reads is printed by `tools/round.sh <kind>`, and `doc/HANDOVER.md`
is the index a round opens for a file that list did not give it. Nothing here is a lesson for a
round; those are traps and habits. Where a number can be printed, the command is given instead.

## 1. Where things stand

- **`main` is `5ded4916`**, batch one hundred and three (sessions 1663–1670), fast-forwarded,
  installed into `target/` (`target/installed-from` names the commit) and pushed by the owner. CI
  ran on it and is **red in three platform jobs**, read from `gh run view <run> --log-failed` with
  the project's token (`tools/state.sh main-checkout` prints the run and the failing steps):
  - **`build (Windows)`** and **`window (WinUI 3)`** fail on the same compile: `crates/pdf-transform/
    src/confined.rs` lines 538 and 1078 call `FileBytes::descriptor` (a Unix file descriptor,
    `pdf_syntax::file`) and `File::from(ReceivedDescriptor)`, neither of which exists on Windows.
    The confined transform worker (ADRs 2140, 2141) was written and tested on Linux only; it needs
    a `cfg(unix)` arm with a stated Windows answer (the transform suite running unconfined there,
    said out loud, or a Windows worker). The `window` job cannot judge the C# until this compiles.
  - **`build (macOS)`**: `crates/pdf-sandbox/tests/encode.rs`'s
    `the_largest_grid_the_budget_admits_encodes_under_the_workers_ceiling` asserts
    `confinement().is_enforced()`, which is seccomp and Landlock — Linux only. The assertion (or
    the test) wants a platform gate, with the macOS answer named.
  - These three are the first job of the next batch's host slot, before anything else it does.
- **The campaign is paused.** No worktree is open (`git worktree list` shows only the main
  checkout), no round is running, no batch branch exists. The brief for the next batch is written
  (section 5) and nothing has been started from it.
- **The owner's list** is `tools/state.sh main-checkout`, never this file. At the time of writing
  it names: the uncommitted answer `A562` to commit (the owner's; no round commits it), the open
  questions `Q550` and `Q559`, two `zune-image` upstream reports to file, the agent's cgroup to
  bound, the read fuzz artefacts to remove, and sccache's ceiling to decide.
- **Open questions a batch is waiting on**:
  - `Q559` — may a folding stroke's chords be coarser (1/20 px) so its exact measure costs under a
    fifth, with the correctness gate restated against the flattened set? This is the follow-up to
    the owner's `A538` ("agreed with wait"); round 1664 found neither of ADR 2122's levers can
    reach the bar (ADR 2164). Both constructions are kept as patches (section 6).
  - `Q550` — whether an archived file embedding a Foxit face carries its BSD notice.
  - `Q562` is answered on the disk (`A562`: the classification is the line; `refuse` stays the
    default until every window has the menu entry, then `ask`) and owes the menu entry.

## 2. The loop, and where it is written

The loop is in the tree, not in anyone's memory; read these in this order before running a batch.

| what | where |
|---|---|
| the batch loop, step by step, with the reasons | `doc/todo/02-every-round.md` section 8 |
| the commands, in the order a merge runs them | `tools/batch.sh`'s header (`open`, `arms`, `export`, `gates`, `check`, `commit`, `install`, `close`) |
| the brief's fixed shape | `doc/todo/_brief-template.md` — common part at most sixty lines, a slot at most twenty-five, `wc -l` both |
| the kinds a round can be, and what each reads | `tools/round.sh --list`; `ROUND_SESSION=<n> tools/round.sh <kind>` |
| what the owner does after a merge | `doc/environment.md`, "After a merge"; `tools/state.sh main-checkout` |
| the rules every round reads | the rule block opening `doc/environment.md`; `doc/todo/02` section 0; `doc/traps/every-round.md`; `doc/habits/every-round.md` |
| a round's one record | `doc/history/README.md` — at most forty lines, a `**Gates.**` paragraph with real figures (the `records` gate holds it) |
| the batch's clock, read back | `tools/state.sh batches` reads the `Round durations` paragraph of each batch commit; `gates-cost` says which gate is dear |

## 3. How a batch was actually run, as practised through batch one hundred and three

The owner's account is `cl`; the rounds run as the agent user `AI`, whose home `/home/AI` the
owner cannot read (`doc/environment.md`, "The machine, the account and the display"). The
orchestrator's scaffolding lives there, one set per batch, named by the batch's first session:

| file | role |
|---|---|
| `/home/AI/batch-<first>-brief.md` | the brief, from the template; eight slots since batch ninety-two |
| the session scratchpad's `prompts-<first>.md` | one launch prompt per round, each naming the brief, the slot, the arms, the ADR and Q numbers |
| `/home/AI/arms-<first>/` | HEAD's six corpus arms by page, exported by `tools/batch.sh arms`; its `README` ends with `done` when complete |
| `/home/AI/chain-<first>.sh` | the merge chain: stage the population, `tier1-1219.sh`, `tools/batch.sh gates`, `tools/batch.sh check`; verdicts into `chain-<first>.done` |
| `/home/AI/tier1-1219.sh` | tier 1 whole: `cargo fmt --all --check`, clippy with warnings as errors, nextest, `cargo test -p conformance`, the fuzz workspace checked on stable and on nightly, `tools/state.sh docs` |
| `/home/AI/msg-<first>.txt` | the commit message skeleton: title line, one bullet per round, `merge:`, `Ledger:`, `Round durations`, `Gates:` |
| `/home/AI/reports-<first>.md` | the orchestrator's notes on each report as it arrives |
| `/home/AI/place-<first>.py` | the one script that edits `doc/traps/README.md`, the trap group files and `doc/habits/measuring.md` — a trap or habit a report proposes is staged here and placed once, before the chain |

The sequence, each step its own command with its output read:

1. `tools/batch.sh open batch-<first>-<last>` from the main checkout. It warms the build and
   starts the arms export; **launch nothing until `/home/AI/arms-<first>/README` ends with `done`**
   — the arms are HEAD's and must be taken on a quiet tree.
2. Write the brief from the template while the build warms: every slot with its contract, the
   premise's evidence checked in the text or code first, the owned files, the gates owed, a wall
   budget (three hours, four for a named build), and its numbers — two ADRs and one Q per slot,
   assigned in the brief so no two rounds take the same number. Allocation since the owner's
   directions of 2026-10-14 to 2026-10-19: one archive (PDF/A remedy) slot, one or two pixels
   slots, one or two host slots (A287's WinUI 3 window), one clause slot that works a `partial` or
   `departed` ledger row, one docs slot, and **one Sonnet 5.5 slot of kind docs that fixes no
   bug** (the owner's words are in `doc/todo/74`'s brief slot; bugs it meets are reported). All
   other rounds run on Opus 5.5.
3. Launch the eight rounds in one message. Each prompt names the brief, its slot, the arms, the
   kind, the session and ADR and Q numbers, and the waiting rule (`tools/wait.sh`, one wait per
   call, never end a turn "waiting").
4. As each report arrives: note it; verify a quoted clause in `doc/md/`; fill the round's bullet
   and its duration and tool uses from the harness notification; stage a proposed trap or habit
   in the place script if it is one (an incident with a rule), else leave it in the notes.
5. When all are in: run the place script; check every script a round edited parses (`bash -n`);
   `cargo check` the touched crates; fill the title, merge and ledger lines; run the chain in the
   background; read `chain-<first>.done`. A red gate is diagnosed before anything is re-run: the
   two commonest causes have been a sibling's mid-edit (compiles now) and a band the machine's
   load moved, which is settled by an A/B against HEAD in one `--clock` hold (habit 101), never
   by re-taking the band on the hold's own runs.
6. Fill the gates line; `tools/batch.sh commit /home/AI/msg-<first>.txt` in the worktree; then
   from the main checkout, as three separate commands, `git merge --ff-only <branch>`,
   `tools/batch.sh install`, `tools/batch.sh close <branch>`; then prune
   `/home/AI/cargo-target/pdf-viewer-batch/debug` and the round exports `r<session>-*` under
   `/home/AI/cargo-target/` before the next `open` (`doc/environment.md`'s hundred-gigabyte rule).
7. **Commit only, never push** (the owner, 2026-09-07). Never `git stash`; never an unscoped
   `cargo fmt` while rounds run; never edit the main checkout except through this recipe (the
   exceptions: committing question files, removing an untracked copy identical to a committed
   one). Never commit `doc/adr_revisit/`.

Three rules this orchestrator learned the hard way, kept here because they are the orchestrator's
and not a round's:

- A new `#[ignore]`d gate test is registered in four places or `state_sections` and
  `sandbox_gates` go red at the merge: its line in `tools/batch.sh gates`, a `doc/todo/02`
  section 2 line, a `tools/state.sh` section and its `all=` entry, and `require_the_sandbox()` or
  a `// no sandbox worker:` comment.
- A hand-off patch from a round is applied with context or by an asserted anchor, never with
  `--unidiff-zero` after siblings moved lines (trap 220), and the tree is compiled before the chain.
- Only the place script edits tracked trap and habit files. An ad-hoc edit once appended a script's
  source to `doc/habits/measuring.md`.

## 4. The owner's standing directions that bind the next batch

Dated, the owner's own words where they were given; the `A` files in `doc/questions/` are the
record for anything decided there.

- **2026-10-14**: "continue without any restrictions… Continue improving performance (and fixing
  bugs) until you reach diminishing returns. Then switch to code improvements/refactoring (bug
  fixes are ok as well) rounds until I say otherwise." The switch was made; batches since are
  code improvement.
- **2026-10-15**: "Please continue with pdf/a remedies in parallel future rounds." — the archive
  slot, every batch, ranked by `tools/state.sh remedies` and `doc/todo/72`.
- **2026-10-19**: one Sonnet 5.5 code-quality slot per batch, god classes split by concern
  (`doc/todo/74`, `tools/state.sh gods`) or a teaching document; "Do not fix any bugs."
- **`A287`** (2026-10-19): the reference host stays winit; a Windows-native .NET host is added; the
  toolkits keep a device; macOS is kept in mind, not built. **`A501`** (2026-10-20) chose WinUI 3.
  The window is `hosts/windows/`, its road map `doc/rfc/0010`'s N3a–N4 rows.
- **2026-10-20**: a round went through the files every subagent reads and compacted them (ADR
  2110), and a round verified prompt caching (nothing changes in how rounds are launched).
- **2026-10-21**: "please do not start a new batch." Batch one hundred and four is written, not
  started.
- **The GPU loop** (`tmp/run-on-gpu.sh`, running as the owner) is for display and adapter
  measurements and probes only — the owner's capitals, 2026-10-10: "DO NOT USE THE LOOP FOR
  ANYTHING ELSE! (only to measure performance!)" — and may unlock the session only through
  `loginctl unlock-session`. `doc/environment.md` carries the rule.
- **One `partial` row per batch** (2026-10-08): the clause slot names it; the commit's `Ledger:`
  line reports it.

## 5. What is ready for the next batch, and what it must take first

The brief is `/home/AI/batch-1671-brief.md` (sessions 1671–1678, ADRs 2178–2193, Q566–Q573;
`ls doc/history | tail -1`, `ls doc/adr | tail -1` and `ls doc/questions/Q* | tail -1` confirm the
next free numbers). Its eight slots, one line each:

1. **archive** — ISO 19005-2 6.2.11.2's widths-array length checked and remedied (restate
   `/LastChar` for a short array, truncate a long one, ADR 2163 section 2); the ten fonts the
   embedding proof draws differently, named one by one; `316267.pdf`'s second verdict.
2. **pixels** — `pdf-font`'s two defects found by round 1663: a CFF `FontMatrix` that is not a
   thousandth is read at a whole-number scale (`cff::units_per_em`), and widths are read past
   `/LastChar` (`metrics::stated_widths`); both from the clauses, with the six arms.
3. **pixels** — the read-back's staging buffer and bind group kept across frames (ADR 2150 left
   the warm page at 0.8–0.9 ms per read back), and one demultiply rounding rule for a medium-less
   page on both paths.
4. **host** — **first, CI's three red jobs on `5ded4916` (section 1)**; then the C# check made a
   script under `tools/` (section 6), then N3a's panels and cards, then `Q562`'s menu entry.
5. **clause** — the press table's other silent poisons (`Presses::rgb_route`, the counters);
   §11.7.5.3's report narrowed to a painted colour; the departures' remaining stale sentences.
6. **pixels** — the vertex stage draws only the admitted runs (an index buffer per zoom step), and
   the replay's census of pages whose steps walk lines (ADR 2134 section 5, `doc/todo/75`).
7. **docs** — `pdf-archive`'s rustdoc at zero and the held list grown crate by crate toward the
   whole workspace; `cpp` added to the two history gates' extensions for `window.cpp`'s two lines.
8. **Sonnet docs** — the thirteenth god class, `doc/todo/74`'s next entry.

Before launching: `tools/batch.sh open batch-1671-1678` re-exports the arms (those under
`/home/AI/arms-1663/` are `66c6ef23`'s, one commit stale); and the brief's slot 4 gets the three CI
causes above written into its contract.

## 6. Kept outside the tree, and why

All under `/home/AI`, readable by the agent account only:

- `/home/AI/dotnet-check/` — a .NET 10 SDK and `cs/check.sh`, which compile `Quorra.Interop` and
  `Quorra.App` on Linux against the Windows App SDK references with warnings as errors, the XAML
  stubbed and packaging off (round 1666, ADR 2168; habit 160). It found two errors CI's first run
  would have stopped on. The next batch's host slot makes it a script under `tools/`.
- `/home/AI/r1664-folding-union/` — two patches against `66c6ef23`: ADR 2122's exact folding
  measure rebased, and the clipped-pieces construction; both pass `git apply --check` there. Which
  one routes, if either, waits on `Q559`. `/home/AI/r1643-folding-union/` is the earlier rebase.
- `/home/AI/arms-<first>/` — one directory per batch back to 1402; the newest is the only one a
  batch needs, the rest are disk.
- `/home/AI/ci-check/` — a detached checkout used for A/B runs against HEAD (re-point it with `git
  fetch /home/cl/projects/pdf-viewer main && git checkout --detach FETCH_HEAD`, then `cargo build
  --release -p pdf-sandbox --bins` there before an A/B).
- `/home/AI/specs/` — the free specification PDFs (ADR 1349); `/home/AI/witness/` — the heavy
  A1 witness, never committed (`doc/third-party-data.md` for its provenance); `/home/AI/hayro-fork/`
  — the owner's fork of hayro (`A227`, `A514`).
- `/home/AI/cargo-target/` — one build directory per purpose (`pdf-viewer` for the main checkout,
  `pdf-viewer-batch` for the worktree, `ci-check`, `nightly-check`, and a round's exports as
  `r<session>-<name>`); `tools/state.sh disk` prints it against the rule.
- The orchestrator's memory for a session on the agent account is
  `/home/AI/.claude/projects/-home-cl-projects-pdf-viewer/memory/MEMORY.md`, the index; the running
  log of every batch is `fifty-round-run.md` beside it.

## 7. Threads the reports left open, for later batches

Each is in a `doc/history/` record's "Unfinished" or "Found, not fixed" paragraph; this is the
list, not the argument.

- Anti-aliasing everywhere for the raster library's `Source` coverage past its 8191-pixel range
  (`lerp(dst, src, min(coverage, clip))`), the cost ADR 2166 section 3 accepted instead.
- A font that already has a descriptor still takes the shipped face over a machine face silently,
  as a statement; its census is owed (`doc/todo/72`).
- A folding stroke's measure at twenty-four threads: a boundary shared between the strips that
  cross a mark, which needs a store that outlives one strip (`strips.rs`).
- The docs walk is now `large` and queues behind lane 1; a capped `cargo doc -j` on the small
  lane may keep its peak under the ceiling and wants a cold measurement first.
- Retyping a popup's text and a rich note's formatting runs in the WinUI window; the file chooser
  is N4; the drive step 83 is flaky.
- `doc/running-the-viewer.md` still dates features in prose no gate sees.
- The GPU carve-out is full and the first-page figures decline with it — a question for the
  owner, not yet written as a `Q`.

## 8. If a quota or a crash ends a session mid-batch

`doc/todo/02` section 8 step 7 is the rule. In practice: the worktree's edits survive; `git
status` in `/home/AI/pdf-viewer-rounds` shows what each round left; a round's own record under
`doc/history/` says whether it finished; a fresh round on the same slot takes the files over
from there rather than starting again. Nothing on `main` is touched until the chain is green.


## Current handover: batch 108 (2026-10-22)

Read [the batch108 supplement](orchestrator-handover-batch108.md) before using
the historical state above. The owner requested closing this batch and stopping;
**do not start new rounds without a new instruction**. Session 1702 was not started.
The supplement records completed work, stability findings, remaining UI issues,
38/39 canonical gates (launch performance remains failed), and final receipt paths.
Ignore historical model names; do not change model. The existing text above is preserved.
