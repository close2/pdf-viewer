# Hand-over: orchestrating the rounds

Status: **standing** — the layer *above* a round, for whoever runs the next batch: the owner, or a
fresh session on the agent account. What a round reads is printed by `tools/round.sh <kind>`, and
`doc/HANDOVER.md` is the index a round opens for a file that list did not give it. Nothing here is
a lesson for a round; those are traps and habits. It is written as *what is* (ADR 1023): where a
number can be printed the command is given instead, and what a batch did is its commit's and its
records' under `doc/history/`, never this file's. One handover, because a second is a copy nothing
checks (ADR 1036's reason).

## 1. Where things stand, and the commands that say it

- **`main`** is what `git -C /home/cl/projects/pdf-viewer log -1 --oneline` prints; what is
  installed is `target/installed-from`, which names the commit and the digest of every installed
  file. A batch is open when `git worktree list` shows `/home/AI/pdf-viewer-rounds` on a
  `batch-<first>-<last>` branch; its brief is `/home/AI/batch-<first>-brief.md`.
- **CI on `main`** is the last line of `tools/state.sh main-checkout`, which reads the run with
  the project's token; a red job is read with `gh run view <run> --log-failed`.
- **The owner's list** is `tools/state.sh main-checkout`, numbered, each step with its command —
  answers on the disk to commit, upstream reports to file, the agent's cgroup, fuzz artefacts the
  tree has read, sccache's ceiling. Never this file.
- **Open questions** are `doc/questions/Q*.md` with no `A` file of the same name, in the main
  checkout as well as the tree (`main-checkout` counts both). An answer the owner wrote on the disk
  is the owner's to commit; no round commits it.
- **The launch gate `t2-launch_path`** has been red at every merge since batch one hundred and six,
  merged over with the judgment written down each time (ADRs 2209, 2221, 2233); the batch that
  makes it green says so in its commit. `/home/AI/launch-ab-1698/` and `/home/AI/launch-frozen-1698/`
  hold the balanced comparison and the frozen binaries; repeating those runs for a green result is
  not the remedy.

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

## 3. How a batch is run

The owner's account is `cl`; the rounds run as the agent user `AI`, whose home `/home/AI` the
owner cannot read (`doc/environment.md`, "The machine, the account and the display"). The
orchestrator's scaffolding lives there, one set per batch, named by the batch's first session:

| file | role |
|---|---|
| `/home/AI/batch-<first>-brief.md` | the brief, from the template; eight slots |
| the session scratchpad's `prompts-<first>.md` | one launch prompt per round, each naming the brief, the slot, the arms, the ADR and Q numbers |
| `/home/AI/arms-<first>/` | HEAD's six corpus arms by page, exported by `tools/batch.sh arms`; its `README` ends with `done` when complete |
| `/home/AI/chain-<first>.sh` | the merge chain: stage the population, the tier-one script, `tools/batch.sh gates`, `tools/batch.sh check`; verdicts into `chain-<first>.done` |
| `/home/AI/tier1-1219.sh` | tier 1 whole: `cargo fmt --all --check`, clippy with warnings as errors, nextest, `cargo test -p conformance`, the fuzz workspace on stable and on nightly, `tools/state.sh docs`. `/home/AI/tier1-1698.sh` is a stricter variant — each walk under `tools/bounded.sh --lock`, the doctests, the fuzz workspace's `fmt` and clippy, one failure flag — and which one a chain runs is the orchestrator's choice, stated in the commit's `Gates:` line |
| `/home/AI/msg-<first>.txt` | the commit message skeleton: title line, one bullet per round, `merge:`, `Ledger:`, `Round durations`, `Gates:` |
| `/home/AI/reports-<first>.md` | the orchestrator's notes on each report as it arrives |
| `/home/AI/place-<first>.py` | the one script that edits `doc/traps/README.md`, the trap group files and the habit files — a trap or habit a report proposes is staged here and placed once, before the chain |

The sequence, each step its own command with its output read:

1. `tools/batch.sh open batch-<first>-<last>` from the main checkout. It warms the build and
   starts the arms export; **launch nothing until `/home/AI/arms-<first>/README` ends with `done`**
   — the arms are HEAD's and must be taken on a quiet tree.
2. Write the brief from the template while the build warms: every slot with its contract, the
   premise's evidence checked in the text or code first, the owned files, the gates owed, a wall
   budget (three hours, four for a named build), and its numbers — two ADRs and one Q per slot,
   assigned in the brief so no two rounds take the same number (`ls doc/history | tail -1`,
   `ls doc/adr | tail -1` and `ls doc/questions/Q* | tail -1` give the next free ones). The
   allocation under the owner's directions (section 4): one archive slot, one or two pixels slots,
   one or two host slots, one clause slot that works a `partial` or `departed` row, one docs slot,
   and one Sonnet 5.5 docs slot that fixes no bug. All other rounds run on Opus 5.5.
3. Launch the eight rounds in one message. Each prompt names the brief, its slot, the arms, the
   kind, the session and ADR and Q numbers, and the waiting rule (`tools/wait.sh`, one wait per
   call, never end a turn "waiting").
4. As each report arrives: note it; verify a quoted clause in `doc/md/`; fill the round's bullet
   and its duration and tool uses from the harness notification; stage a proposed trap or habit
   in the place script if it is one (an incident with a rule), else leave it in the notes.
5. When all are in: run the place script; check every script a round edited parses (`bash -n`);
   `cargo check` the touched crates; fill the title, merge and ledger lines; run the chain in the
   background; read `chain-<first>.done`. A red gate is diagnosed before anything is re-run: the
   two commonest causes are a sibling's mid-edit (compiles now) and a band the machine's load
   moved, which is settled by an A/B against HEAD in one `--clock` hold (habit 101), never by
   re-taking the band on the hold's own runs.
6. Fill the gates line; `tools/batch.sh commit /home/AI/msg-<first>.txt` in the worktree; then
   from the main checkout, as three separate commands, `git merge --ff-only <branch>`,
   `tools/batch.sh install`, `tools/batch.sh close <branch>`; then prune
   `/home/AI/cargo-target/pdf-viewer-batch/debug` and the round exports `r<session>-*` under
   `/home/AI/cargo-target/` before the next `open` (`doc/environment.md`'s hundred-gigabyte rule).
7. **Commit only, never push** (the owner, 2026-09-07). Never `git stash`; never an unscoped
   `cargo fmt` while rounds run; never edit the main checkout except through this recipe (the
   exceptions: committing question files, removing an untracked copy identical to a committed
   one). Never commit `doc/adr_revisit/`. A batch may also commit at each reviewed integration
   point before its batch commit, as batches one hundred and four to one hundred and eight did;
   the whole chain still runs before `main` is fast-forwarded.

Three rules that are the orchestrator's and not a round's:

- A new `#[ignore]`d gate test is registered in four places or `state_sections` and
  `sandbox_gates` go red at the merge: its line in `tools/batch.sh gates`, a `doc/todo/02`
  section 2 line, a `tools/state.sh` section and its `all=` entry, and `require_the_sandbox()` or
  a `// no sandbox worker:` comment.
- A hand-off patch from a round is applied with context or by an asserted anchor, never with
  `--unidiff-zero` after siblings moved lines (trap 220), and the tree is compiled before the chain.
- Only the place script edits tracked trap and habit files. An ad-hoc edit once appended a script's
  source to `doc/habits/measuring.md`.

## 4. The owner's standing directions

Dated, the owner's own words where they were given; the `A` files in `doc/questions/` are the
record for anything decided there.

- **2026-10-14**: "continue without any restrictions… Continue improving performance (and fixing
  bugs) until you reach diminishing returns. Then switch to code improvements/refactoring (bug
  fixes are ok as well) rounds until I say otherwise." The switch is made; batches are code
  improvement.
- **2026-10-15**: "Please continue with pdf/a remedies in parallel future rounds." — the archive
  slot, every batch, ranked by `tools/state.sh remedies` and `doc/todo/72`.
- **2026-10-19**: one Sonnet 5.5 code-quality slot per batch, god classes split by concern
  (`doc/todo/74`, `tools/state.sh gods`) or a teaching document; "Do not fix any bugs."
- **`A287`** (2026-10-19): the reference host stays winit; a Windows-native .NET host is added; the
  toolkits keep a device; macOS is kept in mind, not built. **`A501`** (2026-10-20) chose WinUI 3.
  The window is `hosts/windows/`, its road map `doc/rfc/0010`'s N3a–N4 rows.
- **2026-10-22**: "Another process has worked on this repository …, please use one round to take a
  look at what has been done, and if it meets our standards." — batch one hundred and nine's
  review slot; its record says what it found.
- **The GPU loop** (`tmp/run-on-gpu.sh`, running as the owner) is for display and adapter
  measurements and probes only — the owner's capitals, 2026-10-10: "DO NOT USE THE LOOP FOR
  ANYTHING ELSE! (only to measure performance!)" — never a build, an install, a file the agent user
  cannot read, or a way out of the agent account; it may unlock the session only through
  `loginctl unlock-session`. `doc/environment.md` carries the rule. A run through it stages its
  binaries as `AI`, holds the clock exclusively, has a finite timeout, checks it owns the window and
  the pid, and ends with Escape and a bounded fallback.
- **One `partial` row per batch** (2026-10-08): the clause slot names it; the commit's `Ledger:`
  line reports it.
- **2026-10-22, at batch one hundred and eight's close**: finish, merge, commit and stop; no round
  or batch starts without a new instruction. Batch one hundred and nine is that instruction's
  successor, given the same day.
- **Never change the model** from a historical model name met in a record; the brief's section 3
  states the models.

## 5. Kept outside the tree, and why

All under `/home/AI`, readable by the agent account only:

- `/home/AI/dotnet-check/` — the .NET 10 SDK and its package cache that `tools/dotnet-check.sh`
  finds when `DOTNET_ROOT` and `PATH` name none (ADR 2184).
- `/home/AI/r1664-folding-union/` — ADR 2122's exact folding measure and the clipped-pieces
  construction as patches against `66c6ef23`; `A559` chose chords at a twentieth, and
  `/tmp/folding-exact` is a detached worktree at `66c6ef23` left untouched beside them.
- `/home/AI/arms-<first>/` — one directory per batch; the newest is the only one a batch needs.
- `/home/AI/ci-check/` — a detached checkout for A/B runs against HEAD (re-point it with `git
  fetch /home/cl/projects/pdf-viewer main && git checkout --detach FETCH_HEAD`, then `cargo build
  --release -p pdf-sandbox --bins` there before an A/B).
- The evidence of batches one hundred and four to one hundred and eight: the whole of it, failures
  included, in `/home/AI/evidence-batch108-1698-1703.tar.zst`, with `evidence-archive-1698.json`
  its member list; the launch comparison (section 1); the real-display trials in
  `/home/AI/gpu-evidence-1703/` and the CPU captures in `/home/AI/gtk-capture-1703-cpu1/` with
  `analyze-gtk-1703.py`, the independent mapping. `/home/AI/prepared-1702/` is a preparation for a
  session that never ran, and authorises nothing.
- `/home/AI/specs/` — the free specification PDFs (ADR 1349); `/home/AI/witness/` — the heavy
  A1 witness, never committed (`doc/third-party-data.md` for its provenance); `/home/AI/hayro-fork/`
  — the owner's fork of hayro (`A227`, `A514`).
- `/home/AI/cargo-target/` — one build directory per purpose (`pdf-viewer` for the main checkout,
  `pdf-viewer-batch` for the worktree, `ci-check`, `nightly-check`, and a round's exports as
  `r<session>-<name>`); `tools/state.sh disk` prints it against the rule.
- The orchestrator's memory for a session on the agent account is
  `/home/AI/.claude/projects/-home-cl-projects-pdf-viewer/memory/MEMORY.md`, the index; the running
  log of every batch is `fifty-round-run.md` beside it.

## 6. Threads the reports left open

Each is in a `doc/history/` record's "Unfinished" or "Found, not fixed" paragraph, or in the ADR
named; this is the list, not the argument.

- §11.7.5.3: the `Indexed` bases other than an immediate `ICCBased` one, the calibrated, `Lab`,
  `Separation` and `DeviceN` sources, `Indexed` process-tint overprint, and the output-intent
  policy (ADR 2234).
- The owner's zoom symptoms — the Qt lag, the GTK blur and deep-zoom drift — are not resolved: the
  next investigation tells source sharpness, a transient stand-in's scaling and the compositor's
  scaling apart, and reproduces native Wayland and the deep-zoom anchor on their own. The real
  adapter's trials refused before capture because the requested pointer warp did not move the X11
  pointer; hardware sharpness is unjudged (ADRs 2229, 2241). A device price can stay a seed while no
  compatible device frame exists (ADR 2196).
- PDF/A: thirteen soft-mask selections remain unjudged under `A61`; `A566`'s
  `substitute-stretch` loss is answered and owes its build.
- Anti-aliasing everywhere for the raster library's `Source` coverage past its 8191-pixel range
  (`lerp(dst, src, min(coverage, clip))`), the cost ADR 2166 section 3 accepted instead.
- A font that already has a descriptor still takes the shipped face over a machine face silently,
  as a statement; its census is owed (`doc/todo/72`). `type1.rs::units_per_em` takes an integer
  from its reader as the CFF one did (ADR 2180's "Found, not fixed").
- A folding stroke's measure at twenty-four threads: a boundary shared between the strips that
  cross a mark, which needs a store that outlives one strip (`strips.rs`).
- The docs walk is `large` and queues behind lane 1; a capped `cargo doc -j` on the small lane may
  keep its peak under the ceiling and wants a cold measurement first.
- The WinUI window's file chooser and printing are N4; the drive step 83 is flaky.
- The GPU carve-out is full and the first-page figures decline with it — a question for the
  owner, not yet written as a `Q`.

## 7. If a quota or a crash ends a session mid-batch

`doc/todo/02` section 8 step 7 is the rule. In practice: the worktree's edits survive; `git
status` in `/home/AI/pdf-viewer-rounds` shows what each round left; a round's own record under
`doc/history/` says whether it finished; a fresh round on the same slot takes the files over
from there rather than starting again. Nothing on `main` is touched until the chain is green.
