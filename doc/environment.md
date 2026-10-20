# The environment, and the agreements that go with it

Moved here so that `CLAUDE.md` holds only principles and `doc/HANDOVER.md` only an index.
**Read this before running anything**: the machine, the user the agent runs as, what it
can and cannot open a window on, and where the build lands.

## The rules, one line each — the part of this file every round reads

Every rule below is an incident with an argument, and the argument is in the section the rule
names; a round reads this block, and opens the section only where a line bites (ADR 1639).

- **One heavy walk on the machine at a time**, or two on the lock's two lanes, and you wait for
  it: `ulimit -u 8192; tools/bounded.sh --lock --round <session> --tree <GiB> -- <command>` —
  the wrapper, never a bare `flock`, so the wait is a line of `/home/AI/heavy-walk.log` (ADR 1646)
  and the descriptor stays with the wrapper, which hands the command `HEAVY_WALK_HELD_BY` (ADR
  1674); a locked command runs at four rayon threads unless the caller set `RAYON_NUM_THREADS`
  (ADR 1766); `--data` never above 12 GiB. **The outermost wrapper declares the walk's kind**
  (ADR 1684): `--tree 6` or less is a small walk, one that ends within 600 s, and may run on the
  second lane beside another; a larger tree takes the first; `--clock` is a run whose verdict is
  a time — a band, a floor, an A/B, a reference program on a budget — and runs alone, the second
  lane still granting small walks while it waits for the first (ADR 1869); `--long` is a run whose
  length is its own choice — a fuzz campaign, a seed census, any walk past 600 s — and holds the
  second lane only (ADR 1756). Walks are granted in the order they asked, and a wait stopped
  before its grant is a line too, `lane=-` (ADR 1790).
- **What is not a walk runs bare**: `cargo test -p conformance` and crate-scoped checks take no
  lock (habit 107); a `tools/state.sh` section takes the lock itself, each walk in the lane it
  declares, so it runs as `tools/state.sh --round <session> <section>` (ADR 1698);
  `tools/state.sh gates-cost` names each command a batch ran under the lock, with its queue
  (ADR 1718).
- **A corpus gate builds the sandbox worker inside its hold**, as the wrapper's own `--build
  '--profile gates -p pdf-sandbox --bins'` (`--release` for a release walk), never as a build
  before it (trap 109, ADR 1710); never a tool that forks per package or per input without the
  bound (trap 116).
- **A wait returns within 240 s**: a command that can run longer starts as `tools/wait.sh --start
  <log> -- <command>` and is waited for with `tools/wait.sh <log>` under a harness timeout of
  270 000 ms, re-issued while it exits 75; no wait-loop of a round's own runs past 240 s, and one
  Bash call and one response each carry one wait (ADRs 1976, 1977; traps 204, 208). A longer call
  outlives the five-minute prompt cache, and the next request writes the whole context again.
- **No `git stash`, no `git checkout -- .`, no `git restore .`, no `git add -A` or `-u`, no
  whole-file `cp` restore**: name the paths you mean. A before-and-after is a patch of your own.
- **Format only your own files**, one by one: `rustfmt --edition 2024 <file>`; never `cargo fmt --all`
  and never a binary root's `#[path]` children.
- **Kill by pid or process group you hold**, never `pkill -f`, never `pkill -x <shared name>`,
  never a `pgrep -f` wait-loop on your own command line. Exit 144 with intact output is a neighbour.
- **Scratch goes under `scratchpad/r<round>/`** and nowhere a sibling also writes; nothing of yours
  under the workspace globs (`tools/__pycache__`, a crate under `scratchpad/`): a script under
  `tools/` runs as `python3 -I -B`, because `-I` ignores `PYTHONDONTWRITEBYTECODE` (traps 114, 195).
- **Edit a shared file by targeted replacement, re-read immediately before the write**, never by
  read-whole/write-whole from a copy taken earlier; `assert s.count(anchor) == 1`; read
  `git diff --stat` after a scripted splice.
- **A build error in a crate you were not given is a neighbour mid-edit**: wait and retry; a timing
  failure at load average above 12 is a neighbour building — `uptime` first.
- **Name the worktree in every `git` command** (`git -C <worktree>`), and write a commit message
  to a file for `-F`.
- **Run the viewer under Xvfb only as *The machine, the account and the display* says**; drive a
  host feature in every window before calling it done.
- **A measurement builds its own `--release` binary first** and never runs from the main
  checkout's `target/`; two exported trees are `tools/batch.sh export <session> --as <name>`
  twice, each with its own target directory and `export-sums` (ADR 1890, trap 129).

## Working agreements

- You are running as your own user.  Obviously not a real sandbox, but you do not need to ask
  before deleting files,...   You are not able to modify global config or install anything globally.
  Evaluate if installing something globally by asking the human or creating a user local
  copy / installation automatically is the better choice.
- If a proposed fix looks wrong for this setup, say so instead of running it.
- **The rendering library is this tree's `raster/`**, a sub-project wired in by path
  (`d9bc96b2`), so a change that spans the viewer and the library is one change in one tree, under
  this tree's gates and the library's own (`raster/CLAUDE.md`).
- **The byte-for-byte determinism contract is officially relaxed** (owner, 2026-08-26;
  `raster/doc/adr/0082`): a close best effort from a device lane is good enough, and a
  cross-adapter or CPU-versus-device difference is a tolerance to state and bound. The
  CPU oracle stays the correctness reference; what relaxed is how exactly a device
  lane must match it, never whether §-derived expected values bind.
- Verify claims by running them. Report failures with their output; never assert that
  something works without having checked.
- **A commit that lands on `main` keeps its body.** Four commits once arrived by cherry-pick
  carrying only their title and trailers, and `git log` on `main` was opaque for them. Before
  pushing a pick, `git log -1 --format=%b` must print the body you expect.
- **The process table is shared between parallel rounds, and a pattern names every round's
  processes** (the rule block's *Kill by pid* line). Every round's command lines contain the
  project's path and its programs' names, so `pkill -f` matches a sibling's shell as readily as
  your own: session 604 lost four commands to it, session 605 saw three `cargo build --release`
  return **exit 144** with every artefact intact and called it a harness artefact, and a third
  round's wait-loops died the same way. `pkill -x <name>` narrows the *pattern*, not whose process
  it is — session 800 ran `pkill -x cargo` and took round 799's mid-gate build with it — and a
  `pgrep -f <script>` wait-loop matches its own command line and reports the job running after it
  ended. **Kill by pid, or by the group of a script you started**:
  `kill -- -$(ps -o pgid= -p "$pid" | tr -d ' ')`. A background command launched through the
  harness is **two** groups — the harness's `bash -c` wrapper and the script — so `ps … | grep
  <script> | head -1` picks the wrapper; session 924 killed that, saw the job reported killed, and
  started a second gate sequence beside the first, both appending to one log. **Kill every distinct
  `pgid` the pattern matches, then confirm with a second `ps`** — the failure looks exactly like
  success. A namespace the machine gives every round by one name is one two rounds collide in; the
  stash and the scratchpad below are the same shape.
- **The scratchpad directory is shared too, and not per session** — hence the rule block's
  `scratchpad/r<round>/`. Session 656's `gates.log` was overwritten mid-run by a neighbour writing
  the same name, and three gates were re-run: the file existed, was well-formed, and was somebody
  else's answer.
- **A corpus walk runs through `tools/bounded.sh`, and the bound is the walk's.** On 2026-09-01
  eight survey shards, each a pool of 24 rayon threads, reached a 90 GB working set against 63 GB of
  RAM and a soft lockup the owner ended with the power switch. It was **not a leak** — a shard's peak
  is the same over 340 documents as over 680 — but documents: one fuzzed file's first page costs
  about 11 GB to draw, and the shards met such documents 192 at a time (ADR 0798). On 2026-09-02 a
  `--data 32` survey with no `--tree`, beside two other rounds, took the user slice to 61.09 GB of
  61.9 and the Claude process aborted: `RLIMIT_DATA` is per process, and what mattered was the sum.
  The owner's four rules followed, binding on every round, and the script's header carries the
  timeline: **(1) one corpus walk at a time across all rounds** — the lock's two lanes are how that
  is kept (ADRs 1684, 1756); **(2) `--data` never above 12 GiB for a round; (3) every bounded run
  carries `--tree` — 12 GiB for a walk, 8 for a build — which the script defaults to 12 and without
  which it refuses `--data` above 12; (4) the sum of what a round has in flight stays under
  16 GiB.** `--shards N` gives each of N side-by-side processes `nproc/N` rayon threads (four under
  `--lock`) and a share of the 12 GiB walk budget, at nice 19, and the wrapper's last line names
  the bound that stopped a run rather than the document (trap 18 is why standard error goes through
  a pipe). `--tree` is the same idea for a `cargo build`, whose memory is spread over `rustc`
  processes no single limit sees. `systemd-run --user` is not available to this account, which is
  why the bound is an rlimit and not a cgroup; the cgroup is the owner's to set (ADR 0798).
- **Every heavy command runs under the agent's task budget, `ulimit -u 8192`, because no memory
  bound sees a process count.** On 2026-10-06 an unbounded `cargo-geiger` forked a task per package
  and never waited; the agent's scope climbed to 52 259 tasks, 50 GB resident and 91 GB of swap, and
  the OOM daemon killed the orchestrator and four rounds (trap 116). `RLIMIT_NPROC` refuses the
  `fork` itself, and it counts **every task of the user**, so the figure is the budget of every
  round and the orchestrator together. `tools/bounded.sh` is the one place it is written
  (`tools/bounded.sh --task-budget` prints it, and the `tools/*.sh` that run heavy lines read it
  there); a command the wrapper ran that met it ends on `STOPPED BY THE TASK LIMIT`, and
  `tools/batch.sh check` prints the user's tasks beside the limit. The cgroup that bounds the agent
  itself is the owner's, and `tools/state.sh main-checkout` names it (ADR 1612).
- **`git stash` is shared between worktrees, and a parallel round will take yours.** `refs/stash`
  lives in the common git directory, so every round pushes onto one stack. Session 524 stashed to
  measure a baseline and popped a neighbour's half-finished `pdf-font` edit instead, because `pop`
  takes `stash@{0}`; both trees were wrong and neither said so. A before-and-after is `git diff >
  x.patch`, `git apply -R x.patch`, measure, `git apply x.patch`, plus a copy of any *untracked*
  file. If a stash has already gone wrong, the popped commit is still reachable (`pop` prints its
  SHA): `git checkout --` the files it applied, `git stash store -m "<its message>" <sha>` puts it
  back at `stash@{0}`, and `git stash pop stash@{1}` recovers yours.

  **The one entry on the stack is dead**: `stash@{0}`, a WIP at `ada5411` across `pdf-font`'s
  `loading.rs` and `metrics.rs`, is fully superseded by `b5c1f180`, that round resumed — every one
  of its substantive added lines is in `main`, checked line by line. `tools/round.sh` reports the
  stack non-empty until somebody with the permission runs `git stash drop`, which is right: a
  warning naming a known-dead entry costs one glance here, and a silent stack costs the next round
  whatever the entry turns out to be.
- **A build directory outlives its checkout, and nothing removes it.** `git worktree remove` takes
  the checkout and leaves its build directory behind; twenty-three of them once reached 425 GB
  before anybody looked. A batch's rounds share one worktree and one build directory
  (`tools/batch.sh`, below); a single round's worktree is `tools/worktree.sh open NNN`, and
  `close NNN` takes the checkout and its build directory away as one act. `tools/worktree.sh list`
  names every directory under the build root, whose each one is, and what they add up to (ADR
  0752), which is the figure `doc/todo/02` §5a's threshold is about.
- **A commit message quotes code, and `git commit -m "…"` runs every backtick pair in it**: a
  message saying *``the hazard `git add doc` springs``* executed `git add doc`. **Write the message
  to a file and use `-F`**; `--amend -F` repairs one that already landed.
- **Name the worktree in every `git` command.** Session 614's working directory moved, with no
  `cd`, into a parallel round's tree, and its next `git commit --amend` rewrote that round's commit.
  With `-C` on every command no cwd can move under it.
- **No blanket stage — `git add -A`, `-u`, or `git checkout -- doc`.** A worktree reaches the
  submodules through *symlinks* into the main checkout (`git ls-files -s | awk '$1==160000'` lists
  them), so `-A` and `-u` alike stage each as a 120000 blob in place of its gitlink — invisible in
  `git status --short`, visible as `mode change 160000 => 120000` in `git commit`'s own summary.
  Session 614 hit it with `-A`, session 683 with `-u`, having read a rule that named only `-A`.
  `cargo test -p conformance`'s `every_declared_submodule_is_still_tracked_as_one` catches it and
  prints the repair; in a worktree the repair is `git rm --cached` then `git update-index --add
  --cacheinfo 160000,<sha>,<path>`, never an `rm -f` of the symlink, which takes the submodule away
  from every gate that walks it. `git checkout -- doc` is a third way in (session 720): a directory
  argument restores each submodule path under it as an empty directory, and `pdf-spec`'s build
  script then panics about a missing Arlington model with advice that would clone a second copy;
  the repair is `rmdir` and `ln -s /home/cl/projects/pdf-viewer/doc/<each> doc/<each>`.
  `tools/worktree.sh` and `tools/batch.sh` set `--skip-worktree` on each linked submodule, and that
  is a second line, not a replacement: session 683 was bitten with the flag on. **Name the paths you
  mean**, and read `git commit`'s file list rather than trusting the command that produced it.

## The machine, the account and the display

**Arch Linux. GPU: AMD Strix (Radeon 880M/890M, RDNA 3.5) — RADV. Session: X11.** The agent runs
as user `AI` via `sudo -u AI`, reaching `/home/cl/projects/pdf-viewer` through the `coders` group.
**Hand a run on the real GPU to the user; everything else is testable here.**

- **That sentence is about a *window*, not the adapter.** A **headless** quorra device needs no
  display, no session and no X authority cookie, and `render_raster::options()` names no adapter, so
  `QuorraRasterizer::new_headless` comes up on the **real** Radeon 890M for user `AI`, which
  `adapter_description()` prints on every run (ADR 0387); session 552 spent an hour writing down
  that it could not before measuring a zoom frame from a plain `cargo run`. What genuinely needs the
  owner's session is the swapchain, the present and the cadence. `new_headless_software` pins
  llvmpipe on purpose, for a gate that must not depend on hardware.

- **Two classes of core, and every duration taken here is affected by which one it got.** The
  processor is an AMD Ryzen AI 9 HX 370 — four Zen 5 cores and eight denser Zen 5c, twenty-four
  threads — and the split is a file:

  ```sh
  for c in /sys/devices/system/cpu/cpu*/cpufreq/cpuinfo_max_freq; do
      printf '%s %s\n' "${c%/cpufreq/*}" "$(cat "$c")"
  done | sort -k2 -n
  ```

  What follows for a measurement is `doc/habits/measuring.md`'s: pin with `taskset -c` to the
  fastest class and take the minimum of several fresh processes, or the number is a lottery.
  `crates/viewer-ui/tests/launch_path.rs` derives that list rather than naming it.

- **Five programs confine themselves on this kernel, and each is found beside the program that
  starts it.** `pdf-sandbox-worker` decodes an image, `pdf-view-worker` interprets and draws a
  document, `pdf-vfs-worker` generates a document-as-a-directory, `pdf-script-worker` runs a
  document's scripts, `quorra-transform-worker` runs the transform suite (ADR 2140); each applies a `pdf_sandbox::lockdown::Profile` — seccomp-BPF, a Landlock
  domain that permits nothing, and resource ceilings — before it reads a byte, and its greeting
  reports what the kernel granted. `crates/pdf-sandbox/src/lockdown_linux.rs` is each profile's
  allow-list and the `strace` run it was taken from; a system call missing from one kills the
  worker with `SIGSYS`, which the host names. The script worker needs the engine, so it is built
  only with `cargo build -p pdf-script-worker --features engine --bins`, is beside no installed
  program until a host supplies a level for scripts, and is named elsewhere by `PDF_SCRIPT_WORKER`
  (ADRs 1608, 1609).
- KDE Frameworks 6 packages on Arch have no `kf6-` prefix (`kio`, `kconfig`, `ki18n`).
- **Launch with a login shell** so `umask 002` applies, or every file the agent creates is
  unwritable by `cl`: `sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'`
- **`AI` has no X authority cookie**, so anything needing *the user's* display fails at
  `XOpenDisplayFailed` — but `Xvfb` and `lavapipe` are installed, so the real window, event loop and
  surface all run headless, `xdotool` drives them and `xwd` photographs the result (ADR 0126).
  `tools/drive-windows.sh` is that drive for all four windows, step by step with a verdict per
  step; it is no `cargo` gate, because `Xvfb` and `xdotool` are not build dependencies and a test
  that skipped silently would be worse than none. **A display number is yours only after
  `/tmp/.X<n>-lock` was absent and your own Xvfb's pid holds it** (trap 181). By hand:

  ```sh
  Xvfb :77 -screen 0 900x1100x24 &
  DISPLAY=:77 target/quorra --trace doc/ISO_32000-2_sponsored_EC3.pdf &
  sleep 20   # the window is up long before this, but a long document's title is not
  DISPLAY=:77 xdotool windowfocus --sync $(DISPLAY=:77 xdotool search --name "ISO 32000" | tail -1)
  DISPLAY=:77 xdotool key --delay 400 Right Right Right Right Right
  DISPLAY=:77 xwd -root -silent -out screen.xwd && magick xwd:screen.xwd screen.png
  ```

  What this instrument does, each found by a round it cost time:
  - a document with `/DisplayDocTitle` titles its window with its own title, so a search for its
    file name finds nothing; and ImageMagick here does not sniff xwd from a pipe — `-out` plus
    `magick xwd:<file>` (session 213);
  - **a wheel notch is two events**: `xdotool click 4` is a press and a release, and winit's X11
    backend turns both into `MouseWheel`, so divide before believing a step count (session 214);
  - **a key pressed twice in a row is sometimes one key**: X's auto-repeat detection folds identical
    keycodes close together, so a repeated key needs a delay of a few hundred milliseconds (session
    695, counting a password prompt's bullets);
  - **the pointer has to be inside the window**, 800×1000 on a 900×1100 screen: a `mousemove` past
    it produces no event and looks like a binding that does not work — `xdotool getwindowgeometry`
    first;
  - **`xwd` hands back what the window last painted**, with no compositor to refresh it: session
    638's captures were byte-identical across a rebuild and two GTK renderers. **Send a key, wait,
    then capture**;
  - **there is no window manager** (`mutter` here is Wayland-only), so full screen — a request to
    one, `_NET_WM_STATE_FULLSCREEN` — leaves the window its size in all three toolkits, and
    `xdotool windowactivate`/`getactivewindow` want `_NET_ACTIVE_WINDOW` and say so; what the
    program draws for itself can still be photographed, its window's extent cannot;
  - **`quorra-qt` ignores key presses** until run with `QT_XCB_NO_XI2=1` and clicked once, and wants
    `xdotool windowfocus --sync <id>` before every `key` (ADR 1357).
- **The measurement loop runs as the *owner*, and it is for GPU measurements and nothing else.**
  The owner keeps a loop in their own graphical session which claims `tmp/run-on-gpu.sh`, runs it,
  and leaves `tmp/run-on-gpu.{stdout,stderr}.txt` and `tmp/run-on-gpu.exit` behind;
  `tmp/gpu-loop.alive` ticks each iteration. It exists because a *window* needs the owner's session
  — a headless device does not, so anything measurable headless is measured headless and never
  queued here.

  **The owner's rule, stated by them and absolute: use it only for measurements that require the
  real display or the real adapter. Under no circumstances for anything else — no file access, no
  installs, no fetches, no builds whose output matters, nothing that reaches outside the
  measurement.** The script runs as `cl`, in `cl`'s session, so it can read and write everything
  this account deliberately cannot; that privilege boundary is thin here and nowhere else. A round
  that wants a file the agent user cannot read says so in its report and leaves it to the owner.

  **One thing the loop may do beyond measuring, by the owner's word of 2026-10-14:** unlock the
  owner's session when `loginctl` reads `LockedHint=yes`, with `loginctl unlock-session <id>` and no
  other command — never a typed password, never `xdotool` on the lock screen, never a killed locker
  (trap 174). The script says it did, and a session that stays locked gets no figure.

  What a queued script must do: terminate itself (`timeout N …`, and exit the viewer with
  `Escape` — `SIGTERM` skips the summary), never wait for input, use `./target/…` paths
  (`/home/AI` is unreadable by `cl`), and write only under `tmp/`. `xdotool` cannot reach a
  Wayland client, so force XWayland with `env -u WAYLAND_DISPLAY`.

- **Build directory**: `AI` builds the main checkout into `/home/AI/cargo-target/pdf-viewer` via
  `~/.cargo/config.toml`'s `target-dir`, so the two users never fight over `target/`. Do not "fix"
  this. `pdfref` needs `--work-dir` for the same reason. A worktree that wants a build directory
  of its own names it with `--target-dir` or its own `.cargo/config.toml`, **never** an exported
  `CARGO_TARGET_DIR` (the `sccache` entry below). A batch's worktree builds in
  `/home/AI/cargo-target/pdf-viewer-batch` (ADR 1440), and `tools/batch.sh open` starts
  `cargo build --workspace --all-targets` there detached, logging to `scratchpad/open/build.log`,
  so the rounds find it warm (`BATCH_WARM=0` skips it). Cargo locks a profile's directory, so the
  rounds' `dev` builds queue behind one another rather than duplicating work, while `release` and
  `gates` build beside it under locks of their own (ADR 1451). The directory persists from batch to
  batch, so what a profile pays in a batch is the workspace crates the merge and the siblings
  changed, which no warm-up can save: `open` warms only `dev`, and `tools/state.sh`'s `conformance`
  sweeps run under `dev`, because a release link of that crate costs two minutes after every edit
  and buys tenths of a second a run (ADR 1463).
- **What fills the disk, and what to prune when.** `tools/state.sh disk` prints it and prunes
  nothing: this tree's build directory, the main checkout's, every directory under
  `/home/AI/cargo-target` with its profiles and the day it was last written, `sccache`'s cache
  against its ceiling, the free space under `/home/AI` and the main checkout, and `scratchpad/`
  (ADR 1500). Nearly all of it is `debug`: superseded artefacts stable Cargo has no command to
  collect (`cargo clean --gc` is nightly-only). The pruning is the orchestrator's call, made from
  the main checkout with no round running, each a profile directory by name and never `tmp/`
  (`tmp/pdfref-cache` is the reference renders, a thousand seconds to rebuild — `doc/todo/02`
  section 5a). The batch directory's order is fixed: **`close`, then the prune if `debug` is over
  the rule, then `open`** — `open` starts the warm build that writes there again. Both print the
  directory's size and this line when it is over the rule, and neither runs it (ADR 1526).

  ```sh
  # At a batch boundary — after `close`, before the next `open` — when the batch directory's
  # `debug` passes 100 GB. Costs the next `open`'s warm build, which sccache mostly pays.
  rm -rf /home/AI/cargo-target/pdf-viewer-batch/debug
  # When the main checkout's directory passes 100 GB: its `debug` and `gates` profiles, which only
  # the merge's own runs there rebuild. `tools/batch.sh install` builds in the batch directory's
  # `release`, not here, so pruning this one costs no install.
  rm -rf /home/AI/cargo-target/pdf-viewer/{debug,gates}
  # A directory under the root that no checkout's configuration names and no live round builds in
  # — the line's date says when it was last written, `tools/worktree.sh list` whose it was.
  rm -rf /home/AI/cargo-target/<that directory>
  ```

  `sccache`'s cache is never pruned by hand: at its ceiling it evicts by age, so a full cache
  costs the oldest entries and nothing else. The free-space figure decides whether any of this is
  urgent rather than hygiene; every path above is on the one filesystem.
- **A build script's `env!("CARGO_MANIFEST_DIR")` is baked at *its* compile time, and the shared
  build directory outlives a checkout.** A binary compiled from a checkout that no longer exists
  fails with a message naming a gone path — "data/cmaps is readable: No such file or directory".
  It is not the tree: `touch` the build script's source and rebuild. Which scripts bake it is
  derived, not listed — `tools/round.sh` reads every tracked `build.rs` for the macro and asks the
  newest compiled binary of each (ADR 0752, trap 25).
- **`sccache` is the `rustc-wrapper`, and `export CARGO_TARGET_DIR=…` is what makes it useless.**
  It is set for user `AI` in `~/.cargo/config.toml` (`build.rustc-wrapper`, an absolute path to
  `~/.cargo/bin/sccache`, which is **not on `PATH`** — `which sccache` answers nothing while every
  build goes through it, ADR 0264). **Its Rust cache key includes every environment variable whose
  name begins with `CARGO_`**, so an exported `CARGO_TARGET_DIR` gives a round a private cache
  namespace nothing will read again; the same build with the directory named on the command line
  instead moves the hit rate from nothing to most of it (ADR 0344 has the A/B and the four reasons
  the rest is uncacheable in principle). **Name the target directory on the command line or in a
  `.cargo/config.toml`, never in the environment.**

  `sccache --show-stats` is the instrument and its *categories* are the answer, not its headline
  rate — and its last two lines first: a `Cache size` at its `Max cache size` is a cache evicting
  by age, so a miss there says the entry was pushed out, not that it could never hit (ADR 1463):
  `Cache hits (Rust)` against `Cache misses (Rust)`, and `Non-cacheable reasons` underneath —
  `crate-type` is every binary and every test harness, `multiple input files` is every
  workspace-member `clippy` check. `sccache --zero-stats` only against a server of your own
  (`SCCACHE_DIR=… SCCACHE_SERVER_PORT=… sccache --start-server`): the default one is shared with
  every round, and zeroing it destroys their measurement as well as yours.

  **It must not be allowed to make a measurement**: a round timing a build says which wrapper was
  in place; the program's own timings are unaffected. **Nor must a hypothesis about it**: session
  614 blamed it for a slow Miri run, and was wrong twice — `cargo-miri` sets `RUSTC_WRAPPER` to
  itself, so this wrapper is never in front of the interpreter, and the hour was the *runner*: two
  tests whose inputs are large, an LZW bomb decoding to 7 MB and 1.8 million number lexes, under an
  interpreter four orders of magnitude slower than the processor (ADR 0463). Both now say what they
  do under Miri. **The instrument that answered it was CI's own log** — `gh run view --job …
  --log`, whose per-test timestamps attribute the hour without running anything here.
- **`cargo-fuzz` needs `+nightly`** explicitly; `rust-toolchain.toml` pins the stable toolchain on
  purpose. `cargo-fuzz`, `cargo-deny` and `cargo-nextest` are in the agent's `~/.cargo/bin`, which
  is **not on `PATH`** — `which cargo-fuzz` answers nothing, while `cargo fuzz` runs, because cargo
  looks for a subcommand in its own `bin` directory. Sessions 425 and 426 read the empty `which` as
  "cargo-fuzz is not installed here" and left a target unwritten on the strength of it. **`which`
  answers a question about `PATH`, not about the disk** (ADR 0264).
- The Arlington model is a **submodule**; `pdf-spec` will not build without `git submodule
  update --init`, and **it is the only submodule a build needs.** `doc/pdf.js` is what the
  ratchets are measured over, so CI must have it; `doc/veraPDF-corpus` is what the archive gates
  walk, and they say so and pass where it is absent; the four corpora under `doc/corpora/` are
  optional in the strong sense — no gate names one, and a test naming a path inside one prints that
  it is not checked out and passes. **Two of the four want a sparse clone** rather than the whole
  upstream repository; the recipes are `doc/oracle-and-corpus.md` §2.
- **`tmp/hayro` is a checkout of the whole hayro workspace**, with the project owner's fork as
  `origin` and the maintainer's as `upstream`. **The owner's standing offer is that a fix goes on a
  branch there, they push it and open the pull request, and this tree depends on the fork
  meanwhile** — so a defect in `hayro-jpeg2000` or any other member is a branch to write rather than
  a dependency to wait on. `doc/JPEG2000_FEEDBACK.md` §9 has the detail and the precedent. **This
  changes what a todo file may call blocked**: "waits on the decoder's API" is a statement about
  effort, not about access.

## The specifications, and the one command a fresh clone needs

**The specifications are in this tree encrypted, and that was not engineering.** The ISO and PDF
Association documents in `doc/` and their Markdown conversions under `doc/md/` were once **tracked
in the clear, and the project owner is not licensed to redistribute them** — free to obtain is not
the same permission, and a repository carrying them passes them on to everyone who clones it. They
left the tree, the index and the whole history, and came back **encrypted** (ADR 0187):
`doc/specifications.zip`, ZipCrypto, every specification PDF the gates open and its conversion
(`unzip -l doc/specifications.zip` lists them; ISO 32000-1:2008 joined on 2026-09-21, ADR 1152),
with `.gitignore` covering what `unzip` puts back. `git log --all --name-only` finds no path under
`doc/md/` and no `doc/*.pdf` in any commit, which is the only check worth trusting on this. **This
tree may be published**; nothing else here had to be true first.

**Run this once in a fresh clone, and every gate and example in this tree works:**

```sh
unzip -P "$(cat doc/specifications.password)" doc/specifications.zip    # from the workspace root
```

**The password is `doc/specifications.password`**, one line, gitignored
(`/doc/specifications.password`), readable by the owner and the `coders` group only, on the owner's
machine and nowhere in the tree or its history; a fresh clone gets it from the owner. It is the same
value the `SPEC_ZIP_PASSWORD` repository secret holds, and when the archive is rebuilt the file and
the secret change together: `7z a -tzip -mem=ZipCrypto -p"$(cat doc/specifications.password)"
doc/specifications.zip <the entries>` (Info-ZIP's `zip` is not on this machine; `unzip` reads only
ZipCrypto, so AES is not an option), then a round trip into a scratch directory compared by hash
before the archive is committed.

**Every reference to the documents stays as it was**, decided by the owner: the tests and
measurement examples that open `doc/ISO_32000-2_sponsored_EC3.pdf` or `doc/PDF20_AN001-BPC.pdf`
(`grep -rl` over `crates/*/tests crates/*/examples` finds them) fail loudly until you have, and
`cargo test -p conformance` checks no citation without `doc/md/ISO_32000-2_sponsored_EC3.md`. **CI
is a developer like any other here** and unpacks the archive from the `SPEC_ZIP_PASSWORD`
repository secret before its tests; a pull request from a fork gets no secret, and the step says
so rather than failing obscurely.

## After a merge: the commands the owner runs in the main checkout

A round works in a worktree and never edits `/home/cl/projects/pdf-viewer`, so what a batch cannot
carry is left on the owner's disk. `tools/state.sh main-checkout` (`tools/main-checkout.py`) reads
the main checkout without touching it and prints one line per kind of thing, with its count, and
then **the owner's list**: every one of those that has anything to do, once, numbered in the order a
person does them, each with its command or the files it acts on (ADR 1601). The counts say what is
on the disk and the list says what to do about it; nothing is said in both. This section is what
each line means, in the order it prints them, and nothing a count could say (ADR 1440). A line whose
count is zero has nothing to do, and puts nothing on the list.

```sh
tools/state.sh main-checkout             # first, from the main checkout or the worktree

# `answered, uncommitted: N …, newest first: A171 (2026-10-05) …` — the owner's own answers on the
#   disk and in no commit, each dated by when it landed: a round's list read from tracked files calls
#   their questions open until they are committed, so this line is the one a merge reads first, and
#   `tools/batch.sh check` repeats it (ADR 1588). No round writes or commits them; the owner does,
#   which is the list's "commit the N answer(s)" item.

# `open questions, less those answered on the disk: N: Q254 …; the tracked files alone call M more
#   open: …` — what is open is the first list, and the owner's list names each one's file to answer;
#   the second is what the uncommitted answers above close, and it empties when they are committed.

# `local edits the fast-forward would refuse over: N (paths)` — a local edit to a path the batch
#   changes stops `git merge --ff-only`. The list's first item then sets it aside and puts it back
#   around the fast-forward; never commit it on main first, which ends the fast-forward.

# `fuzz/Cargo.lock: tracked|untracked, agrees with Cargo.lock | pins N version(s) …` — the fuzz
#   workspace's lock is tracked (ADR 1439), so a merge replaces an ignored copy; a version the root
#   lock does not pin is cleared by the merge or by the list's patch item; this names it:
cargo test -p conformance --test fuzz_workspace

# `fuzz/artifacts: N read, M unread, K slow-unit warning(s)` — a read artefact is one the tree
#   names, so a round read it, fixed it and turned it into a test: the list's "remove" item is the
#   one `rm` that takes them all. An `unread:` line beneath stays until a round reads it
#   (doc/verify.md's fuzz block). A slow-unit warning is libFuzzer's clock under the sanitiser, read
#   in a release binary before it is believed, and removed by nothing here.

# `fuzz/corpus: N of M target(s) unseeded: <targets>; K stale by <record>'s census: <targets>` — the
#   list's "re-seed" item is the one command for both, behind the lock, because it walks the corpus.

# `doc/patches: N owed …, M whose base it no longer pins, F for a fork the owner is to create, W
#   waiting …, K stating no base` — a patch owed is a fix to a dependency this tree pins from a fork,
#   written against the `rev` its `Base:` names; a round cannot push to the fork, so the list's
#   "apply" item names every patch for that fork at once: apply them on that base in a clone of the
#   fork and push, move every `rev` the root Cargo.toml pins to that repository to the new commit
#   together, and both locks follow. The count goes when the manifest no longer pins the base (ADRs
#   1447, 1463). A patch to a dependency the manifest takes from crates.io is a fork to create once
#   the question its preamble names is answered on the disk and its `Fork:` is named: the list's
#   "create the fork" item is the one sentence that creates and wires it (ADR 1589), and the day the
#   manifest pins the fork those patches count as applied — as `zune-jpeg`'s two are, the manifest
#   pinning `close2/zune-image` by `rev` (A227, ADR 1730). Each upstream report beside a patch — the `.md`
#   whose head says where it is filed — is the list's "file" item until the owner writes `Filed:
#   <the issue>` at its head (ADR 1601). A `waiting:` line beneath is a patch with no fork to apply
#   it to until the question it names is answered; a `no Repository:/Base: preamble:` line is a
#   patch whose base cannot be read, so whether it is owed cannot be said: write those two lines at
#   its head. For the hayro forks, after the `rev` moves:
cargo update -p hayro-jbig2 -p hayro-jpeg2000 -p hayro-ccitt
cargo update --manifest-path fuzz/Cargo.toml -p hayro-jbig2 && cargo test -p conformance --test fuzz_workspace

# `section signs, uncommitted: N `§` after another standard's name …` — a question file is an
#   instruction document, so its `§` is ISO 32000-2's (ADR 1452), and the main checkout's own
#   `cargo test -p conformance` fails on each place the list's "write the other standard's section"
#   item names, which no worktree run can see. Write the other standard's section in words — "ISO
#   19005-4 section 6.2.7.3", "ISO/TS 32002 section 5.1.3"; the scan reads a paragraph, so a
#   standard named at the end of the line above a sign owns it (ADR 1464). It comes before the
#   commit of the answers, because the question file is committed beside them. Then:
cargo test -p conformance --test documents

# `build directory: <dir>, N GiB against the rule of 100 GiB` — the main checkout's build
#   directory against the build-directory entry's rule above; over it, the list's "prune" item is
#   the `rm` of its `debug` and `gates` profiles, run with no round running.

# `sccache: <cache>, N GiB of a C GiB ceiling` — at its ceiling the cache evicts by age, which
#   costs the warm builds their oldest entries and nothing else, so it is never pruned by hand; the
#   list's "decide sccache's ceiling" item is the owner's choice between raising `size` under
#   `[cache.disk]` and leaving it to evict.

# `agent's cgroup: N scope(s) hold user AI's processes, M with a limit above 16384 tasks or 40 GiB`
#   — the agent runs in a tab's scope of the owner's own session, which no rlimit of this tree
#   reaches; a `scope:` line beneath names each, with the tightest `pids.max` and `memory.max` on its
#   path. 16384 is twice the task budget every heavy command runs under, so the rlimit acts first and
#   the cgroup holds what it cannot see; 40 GiB is ADR 0798's `MemoryMax`. The list's "bound the
#   agent's scope" item writes both into each unbounded tab as the owner, who owns the files, or
#   starts the next session in a scope of its own with `systemd-run --user --scope` (ADR 1612).

# `the owner's list, in the order to do them: N` — the items, numbered: what stops the
#   fast-forward, the question files' `§`, the commit of the answers, the open questions, the forks
#   and the upstream reports, the patches to apply, the agent's scope, the re-seed, the artefacts,
#   and the disk last.
```

