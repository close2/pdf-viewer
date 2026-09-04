# The environment, and the agreements that go with it

Moved here so that `CLAUDE.md` holds only principles and `doc/HANDOVER.md` only an index.
**Read this before running anything**: the machine, the user the agent runs as, what it
can and cannot open a window on, and where the build lands.

## Working agreements

- You are running as your own user.  Obviously not a real sandbox, but you do not need to ask
  before deleting files,...   You are not able to modify global config or install anything globally.
  Evaluate if installing something globally by asking the human or creating a user local
  copy / installation automatically is the better choice.
- If a proposed fix looks wrong for this setup, say so instead of running it.
- **A session may depend on the local quorra checkout during development** (owner,
  2026-08-26): a temporary `[patch."https://github.com/close2/quorra"]` section in
  `Cargo.toml` pointing at `/home/cl/projects/render-lib` is the fast loop for work
  that spans both trees. It is reverted before any commit — CI has no sibling checkout
  — and the committed dependency stays the git pin.
- **The byte-for-byte determinism contract is officially relaxed** (owner, 2026-08-26;
  quorra's ADR 0082): a close best effort from a device lane is good enough, and a
  cross-adapter or CPU-versus-device difference is a tolerance to state and bound. The
  CPU oracle stays the correctness reference; what relaxed is how exactly a device
  lane must match it, never whether §-derived expected values bind.
- Verify claims by running them. Report failures with their output; never assert that
  something works without having checked.
- **A commit that lands on `main` keeps its body.** Four commits arrived by cherry-pick carrying
  only their title and trailers — the pick dropped the message body, and `git log` on `main` is
  opaque for them where every neighbour explains itself. The argument survives in the ADR, but a
  reader of the log should not need to know that. Before pushing a pick, `git log -1 --format=%b`
  must print the body you expect.
- **The *process table* is shared between parallel rounds, and `pkill -f` matches on the path.**
  Every worktree lives under `…/pdf-viewer/.claude/worktrees/rNNN`, so a round running
  `pkill -f pdf-viewer` to clean up its own windows matches **its own shell, and its neighbours'** —
  every command line a sibling round is running contains the project's name because its working
  directory does. The six-hundred-and-fourth session lost four commands to this before it saw the
  pattern; the six-hundred-and-fifth saw three `cargo build --release` invocations return **exit
  144** while producing every artifact correctly and recorded it as a harness artefact under load,
  which was reasonable and wrong; and two of a third round's wait-loops died the same way.

  **Kill your own children by PID, or by their process group** — `kill "$pid"`, or
  `kill -- -$(ps -o pgid= -p "$pid" | tr -d ' ')` for the whole group of a script you started. A
  round knows the pid of everything it launched; that is the only handle on the machine that names
  *your* process and no sibling's.

  **A background command launched through the harness is *two* process groups, and killing the
  first is not killing the job.** The harness wraps the command in a `bash -c` of its own, and that
  wrapper and the script it runs are each a group leader — so `ps … | grep <script> | head -1`
  picks the **wrapper**, whose group contains the wrapper and not the script. The
  nine-hundred-and-twenty-fourth session did exactly that, watched the harness report the job
  killed, and started a second gate sequence beside the first: two runs competed for the machine
  and appended to one log by name, which is how a `nextest` line and a `fixed_documents` line came
  to sit next to each other in a file that only has one of each. **Grep for every match and kill
  every distinct `pgid`**, then confirm with a second `ps` that the pattern is gone — the confirming
  `ps` is the whole guard, because the failure looks exactly like success.

  **`pkill -x <exact-name>` is not the safe form, and this paragraph offered it as one until a
  round was bitten by it.** `-x` bounds the match to the executable's *name*, and says nothing whatever
  about whose process it is — so for a program every round runs under one name (`cargo`, `rustc`,
  `cargo-nextest`, `pdfref`) it is not a narrowing at all. The eight-hundredth session ran
  `pkill -x cargo` to stop its own gate run and took round **799**'s mid-gate build with it, which
  was then watched rebuilding its `gates` profile from near scratch; that round's history file
  records it. `pkill -x` is safe only for a name no sibling round runs, which on this machine is a
  short list and not one worth guessing at. The same shape bites more gently in the other
  direction: a `pgrep -f <script>` wait-loop matches **its own command line** and reports the job
  still running after it has finished.

  So: `pkill -f` against a path is the loud failure, `pkill -x` against a shared program name is
  the quiet one, and neither is a substitute for a pid. If a command returns 144 with its output
  intact, suspect a neighbour before suspecting the harness.

  It is the same shape as the stash and the scratchpad below: **a namespace the machine gives every
  round by one name is a namespace two rounds will collide in.** This one is worse than those two
  because the victim is a *sibling*, so the round that pays is not the round that erred — and `-x`
  narrows the *pattern* rather than the namespace, which is exactly why it reads as a fix.

- **The scratchpad directory is shared between parallel rounds too, and it is not per-session.**
  A round writing `gates.log` there has it overwritten by a neighbour writing the same name, mid-run
  — which happened in the six-hundred-and-fifty-sixth and cost three gates a re-run under a
  uniquely named file. The failure is quiet: the file exists, it is well-formed, and it is somebody
  else's answer. **Name a scratch file after the round** (`gates-656.log`), or put it under the
  worktree's own `tmp/`. Same reasoning as the stash below: anything the harness gives every round
  by the same path is a thing two rounds will collide in.

- **A corpus walk runs through `tools/bounded.sh`, one walk on the machine at a time, and never as
  eight shards of one thread per core.** On 2026-09-01 the rounds' campaign — eight survey shards
  over `batch2/GHOSTSCRIPT`, a census over five batches, gates and builds beside them — reached a
  90 GB working set against 63 GB of RAM, 47 GB into swap, and a soft lockup the owner ended with the
  power switch. Nothing was under a limit and nothing on the owner's side was acting. The
  eight-hundred-and-sixty-sixth round measured it and it was **not a leak**: a shard's peak is the
  same over 340 documents as over 680, and a single-threaded walk is flat from the first document to
  the last. It is a *document*: one fuzzed file whose first page costs about 11 GB to draw, in one
  process, on one thread — and eight shards each running a pool of 24 rayon threads met such
  documents 192 at a time. ADR 0798 has the figures.

  So a walk is put under a bound, and the bound is the walk's rather than the process's:
  `tools/bounded.sh --shards N -- <command>` gives each of N side-by-side processes `nproc/N` rayon
  threads and `32 GiB / N` of `RLIMIT_DATA`, at nice 19, and says afterwards what the run cost or
  what stopped it — a program that runs out prints its own allocation failure and the wrapper's last
  line names the bound rather than the document (trap 18 is why the standard error goes through a
  pipe and is read back). **32 GiB is the whole walk's, and four shards is the cap**: 61 GiB on the
  machine, 16 kept for the owner's desktop, about 12 for a parallel round's gates and build, and
  the rest for one walk. Two rounds do not walk at once, for the reason `doc/todo/02` §2 already
  gives about a loaded machine.

  **That paragraph's 32 GiB took the machine down on 2026-09-02, and four rules replace it.** The
  eight-hundred-and-seventy-fourth round launched `tools/bounded.sh --data 32 -- safedocs survey`
  in the background — the whole walk budget for one 24-thread process, no `--tree` — beside the
  desktop, the Claude process, sccache and two other rounds; the user slice's memory peaked at
  61.09 GB of 61.9, every shell call of two rounds stalled from 09:05, and at 09:08 the Claude
  process aborted. `RLIMIT_DATA` is per process, and 32 GiB was sized for a machine running one
  walk, not three rounds. The owner's rules, binding on every round: **(1) one corpus walk at a
  time across all rounds; (2) `--data` never above 12 GiB for a round; (3) every bounded run also
  carries `--tree` — 12 GiB for a walk, 8 for a build — and `tools/bounded.sh` defaults it to 12
  where none is given, and refuses `--data` above 12 without it; (4) the sum of what a round has in
  flight stays under 16 GiB.** The script's header carries the timeline. `--tree GiB` is the same idea for a `cargo build`, whose memory is
  spread over `rustc` processes no single limit sees. `systemd-run --user` is not available to
  this account and the agent's processes live in the owner's own session scope, which is why the
  bound is an rlimit and not a cgroup; the cgroup is the owner's to set and ADR 0798 says how.

- **`git stash` is shared between worktrees, and a parallel round will take yours.** `refs/stash`
  lives in the *common* git directory rather than in the worktree, so every round running at the
  same time pushes onto one stack. A round that stashed its changes to measure a baseline, and
  popped them back afterwards, got a neighbour's half-finished `pdf-font` edit instead — because
  the neighbour had pushed in between and `pop` takes `stash@{0}`. Both trees were wrong and
  neither said so.

  So **do not `git stash` here**. To take a before-and-after measurement, use a patch of your own:
  `git diff > x.patch`, `git apply -R x.patch`, measure, `git apply x.patch` — plus a copy of any
  *untracked* file, which `git diff` does not carry. If a stash has already gone wrong, the popped
  commit is still reachable (`pop` prints its SHA, and `git fsck` finds it): `git checkout --` the
  files it applied, then `git stash store -m "<its original message>" <sha>` puts it back at
  `stash@{0}` with the stack order restored, and `git stash pop stash@{1}` recovers yours.
  Found in the five-hundred-and-twenty-fourth session.

- **A worktree's *build directory* outlives the worktree, and nothing removes it.** A parallel round
  is given its own `target-dir` in a per-worktree `.cargo/config.toml` — that is right, and it is
  what keeps rounds off one build lock without the `CARGO_TARGET_DIR` export `sccache` cannot see.
  What is easy to miss is that `git worktree remove` deletes the *checkout* and leaves the build
  directory behind, at **19-29 GB apiece**. Twenty-three of them accumulated to **425 GB** across
  five batches before anybody looked, while the number being repeated in reports was a stale one
  taken from a single directory months of rounds earlier.

  So **remove the two together**, and `tools/worktree.sh` is that command — `open NNN …` prepares a
  round's checkout and `close NNN …` takes the checkout and its build directory away as one act.
  `list` names **every** directory under the build root, whose each one is, and what they add up
  to — which is not the same claim it used to make. It listed the directories *it* names,
  `pdfv-rNNN`, so the only thing it could report as orphaned was one of its own; two left by
  another kind of round sat beside them unseen for hundreds of rounds, and when the widening was
  made the directories the old glob could not see were most of the root's size. The total is there
  because `doc/todo/02` §5a's threshold is about the root and no instrument printed it (ADR 0752).
  By hand it is:

  ```sh
  git worktree remove --force .claude/worktrees/rNNN
  git branch -D round-NNN
  rm -rf /home/AI/cargo-target/pdfv-rNNN      # the half `worktree remove` does not touch
  ```

- **A commit message here quotes code, and `git commit -m "…"` in a double-quoted shell string runs
  every backtick pair in it.** A message saying *``the hazard `git add doc` springs``* executed
  `git add doc` and stored the sentence with a hole in it. Nothing was damaged — the substitution ran
  with no pathspec and git refused — but the next one may not be so lucky, and a mangled message is
  the project's own memory losing a word.

  **Write the message to a file and use `-F`.** It costs one line, it survives backticks, `$`, `!`
  and newlines alike, and `--amend -F` repairs one that already landed.

  And the sweep §5a asks for is worth doing by *profile* rather than wholesale: `debug` was **131 GB
  of 158** in the shared directory and is the one Cargo on stable cannot garbage-collect, while
  `release` and `gates` are 9.4 and 4.7 and are what the gates and §5's binaries run from. `tmp/` is
  never swept — it holds the reference-render cache, and deleting it costs the next oracle run about
  a thousand seconds of `pdftoppm`, `mutool` and `gs`.

- **Name the worktree in every `git` command: `git -C /…/worktrees/rNNN …`.** A round's shell
  working directory is not a guarantee. In the six-hundred-and-fourteenth session it moved, without
  any `cd` to a worktree, from `r614` to **`r616` — a parallel round's tree** — and the next
  `git commit --amend` landed on *their* branch, rewriting the commit `round-616` had just made.
  Nothing was lost: the amend had nothing staged, so the tree, the parent, the message and the
  author date were identical and only the committer timestamp and the SHA changed, and the original
  is still in that worktree's reflog. But it could as easily have been an `--amend` with a diff.
  **`pwd` before believing a `git` command, or better, do not depend on `pwd` at all** — every
  command in a parallel round can carry `-C` and then no cwd can move under it.

- **And do not `git add -A` here either, for a neighbouring reason.** A parallel worktree reaches
  the submodules through *symlinks* into the main one, so `-A` sees six paths whose type disagrees
  with the index and helpfully stages the disagreement: the gitlinks become 120000 blobs and the
  commit ships a symlink where a submodule was. It is invisible in `git status --short` afterwards
  and `git restore --staged` does not put it back. `cargo test -p conformance` catches it —
  `every_declared_submodule_is_still_tracked_as_one` prints the six paths and the `update-index
  --cacheinfo` loop that restores them — which is exactly what that gate is for, and the
  six-hundred-and-fourteenth session is the round it caught. **Name the paths you mean**, or run
  the gate before believing a commit.

  **`git add -u` springs it exactly as `-A` does, and this paragraph named only `-A` for
  sixty-nine rounds.** The six-hundred-and-eighty-third session read the rule, added its *new*
  files by name as it says to, and staged the rest with `-u` — a different flag with the same
  consequence, because a gitlink whose working-tree entry is a symlink is a **modification to a
  tracked path** and `-u` is precisely "stage every modification to a tracked path". `git commit`'s
  own summary is where it shows: `mode change 160000 => 120000`. The repair is the loop above and
  costs one `--amend`, but **do not run that recipe's `rm -f "$p"` in a parallel worktree** — those
  symlinks are how the worktree reaches the submodules at all, and deleting them takes
  `doc/pdf.js` away from every gate that walks it. `git rm --cached` followed by
  `git update-index --add --cacheinfo 160000,<sha>,<path>` fixes the index and leaves the working
  tree alone.

  So the rule is about **any** blanket stage rather than about one flag: `git add <path> …`, and
  read `git commit`'s own file list rather than trusting the command that produced it.

  **`tools/worktree.sh` sets `--skip-worktree` on each linked corpus, and that is a second line
  rather than a replacement for the rule.** In isolation the flag holds against every shape the
  six-hundred-and-eighty-fourth session could construct — `add -A`, `add -u`, `add -u doc`, the
  submodule path named directly — and `tools/worktree.sh list` prints, per worktree, whether it is
  on. What it does **not** do is explain the six-hundred-and-eighty-third session, which was bitten
  **with the flag set on all four paths**, and whose worktree still showed `4/4` afterwards. That
  round's branch was clean and nothing reached a commit, so no harm was done and no diagnosis was
  possible after the fact.

  The honest state of it: **the rule above is what protects you, the flag is a belt whose
  sufficiency is disproved by one observation nobody can now reconstruct, and `list` exists so the
  next round to be bitten can tell "the guard is off here" from "the guard is on and something else
  happened" without spending an hour on it.** Whichever it is, `cargo test -p conformance` catches
  the result, which is the only line of the three that has never failed.

  **And `git checkout -- doc` takes the symlinks away**, which is a third way into the same hole and
  the one the seven-hundred-and-twentieth session fell into: it is a *directory* argument, so git
  restores every submodule path under it as an empty directory and the links into the main
  worktree are gone. Nothing in `git status` says so — the gitlinks look untouched — and what a
  round sees instead is `pdf-spec`'s build script panicking with *the Arlington PDF Model is
  missing … It is a git submodule; run: `git submodule update --init`*, whose advice would clone a
  second copy into the worktree rather than restore the link. The repair is
  `rmdir doc/arlington-pdf-model doc/pdf.js && ln -s /home/cl/projects/pdf-viewer/doc/<each> doc/<each>`,
  and the rule is the same one two paragraphs up: **name the paths you mean.** A checkout of nine
  files by name does what a checkout of `doc` was meant to do and touches nothing else.

- **The one entry now on the stack is dead, and is recorded here so that no round spends an hour
  deciding that again.** `stash@{0}` is a WIP on a worktree branch at `ada5411`, 341 lines across
  `pdf-font`'s `loading.rs` and `metrics.rs` — the round that a server-side overload killed in the
  six-hundred-and-forties. **It is fully superseded by `b5c1f180`**, which is that same round
  resumed: all 109 of its substantive added lines are present in `main`'s working tree, checked
  line by line rather than by reading the two commit messages. It does not apply to `main` any
  more and there is nothing in it to recover. `tools/round.sh` will keep reporting the stack as
  non-empty until somebody with the permission runs `git stash drop`, which is the right outcome —
  **a warning that names a known-dead entry costs a round one glance at this paragraph, and a
  silent stack costs the next round whatever the entry turns out to be.**

## The machine, the account and the display

**Arch Linux. GPU: AMD Strix (Radeon 880M/890M, RDNA 3.5) — RADV. Session: X11.** The agent runs
as user `AI` via `sudo -u AI`, reaching `/home/cl/projects/pdf-viewer` through the `coders` group.
**Hand a run on the real GPU to the user; everything else is testable here.**

- **That sentence is about a *window*, and it was read for many rounds as being about the
  adapter.** It is not: a **headless** quorra device needs no display, no session and no X
  authority cookie, and `render_quorra::options()` names no adapter — so
  `QuorraRasterizer::new_headless` comes up on the **real** Radeon 890M for user `AI`, which
  `adapter_description()` prints on every run. Session 552 measured a zoom frame's phases on the
  owner's own hardware from a plain `cargo run`, having spent the first hour of the round writing
  down that it could not. So: **an offscreen measurement is takeable here on the real adapter**, and
  what genuinely needs the owner's session is the swapchain, the present and the cadence.
  `new_headless_software` pins llvmpipe on purpose and stays what it is for — a gate that must not
  depend on hardware.

- KDE Frameworks 6 packages on Arch have no `kf6-` prefix (`kio`, `kconfig`, `ki18n`).
- **Launch with a login shell** so `umask 002` applies, or every file the agent creates is
  unwritable by `cl`: `sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'`
- **`AI` has no X authority cookie**, so anything needing *the user's* display fails at
  `XOpenDisplayFailed`. **The viewer can still be run, and this file said otherwise for dozens of
  sessions** (ADR 0126): `Xvfb` and `lavapipe` are installed, so the real window, the real event
  loop and the real vello surface all work, `xdotool` drives them and `xwd` photographs the
  result.

  ```sh
  Xvfb :77 -screen 0 900x1100x24 &
  DISPLAY=:77 target/pdf-viewer --trace doc/ISO_32000-2_sponsored_EC3.pdf &
  sleep 20   # 1023 pages: the window is up long before this, but the title is not
  DISPLAY=:77 xdotool windowfocus --sync $(DISPLAY=:77 xdotool search --name "ISO 32000" | tail -1)
  DISPLAY=:77 xdotool key --delay 400 Right Right Right Right Right
  DISPLAY=:77 xwd -root -silent -out screen.xwd && magick xwd:screen.xwd screen.png
  ```

  **Two corrections from the two-hundred-and-thirteenth session's run, both of which cost time.**
  `xdotool search --name ISO_32000` finds nothing: that document sets `/DisplayDocTitle`, so its
  title bar reads *ISO 32000-2:2020 (PDF 2.0)…* with a space, which is the feature working. And
  `xwd … | magick - screen.png` fails with *no decode delegate*, because this machine's
  ImageMagick no longer sniffs xwd from a pipe; `-out` plus `magick xwd:<file>` does.

  **A wheel notch is two events here.** `xdotool click 4` is a button press *and* a release, and
  winit's X11 backend turns both into `MouseWheel`, so one `click` is two `Command::Scroll`s or
  two zoom steps. It has always been so — the sidebar's scrolling has doubled the same way since
  it landed — and it is a fact about this instrument rather than about the code: divide before
  believing a step count measured this way. Found in the two-hundred-and-fourteenth session,
  checking Ctrl + wheel in the window.

  **A key pressed twice in a row is sometimes one key here.** `xdotool type --delay 80 zzz` puts
  **two** characters into a text field, and `xdotool key --delay 300 z z z` puts three: X's own
  auto-repeat detection folds identical keycodes that arrive close together, so a repeated
  character is lost. It is the same shape as the wheel note above and the same rule follows —
  **a repeated key needs a delay of a few hundred milliseconds before the count means anything**.
  Found in the six-hundred-and-ninety-fifth session, typing a password into §7.6.4.1's prompt and
  counting the bullets, which is the one interface in this tree that shows a character count and
  no characters.

  **And the pointer has to be inside the window**, which is 800×1000 on a 900×1100 screen: a
  `mousemove` to 850 produces no wheel event at all and looks exactly like a binding that does
  not work. `xdotool getwindowgeometry` first.

  **`xwd` hands back what the window last painted, and there is no compositor to refresh it.**
  Several captures in the six-hundred-and-thirty-eighth session were byte-identical across a
  rebuild *and* across two different GTK renderers, and showed chrome the program had already
  taken away; the picture changed only after a key press forced a repaint. So the recipe is
  **send a key, wait, then capture** — a screenshot taken of a window that has had no reason to
  redraw is a photograph of the past, which is the same failure as a stale binary and reads
  exactly like a change that did not work.

  **There is no window manager here, and some things are requests to one.** `mutter` is installed
  and is Wayland-only in this build (`--x11` is not an option it has), and nothing else on the
  machine is a window manager. Full screen on X11 is `_NET_WM_STATE_FULLSCREEN` — a *request* —
  so `GtkWindow::fullscreen`, `QWidget::showFullScreen` and `winit`'s `set_fullscreen` all
  return with the window the size it was. What can still be photographed is everything the
  program draws for itself: which chrome it hid, which panel it opened, what the page did. What
  cannot is the window's extent. Same for `xdotool windowactivate` and `getactivewindow`, which
  want `_NET_ACTIVE_WINDOW` and say so.

  **This is the only way to exercise the loop** — key press to command to request to frame to
  window — which is where every defect of sessions 140 to 142 lived and which no gate touches.
  Not a gate itself: `Xvfb` and `xdotool` are not build dependencies and a test that skipped
  silently would be worse than none.
- **The measurement loop runs as the *owner*, and it is for GPU measurements and nothing else.**
  The project owner keeps a loop in their own graphical session which claims
  `tmp/run-on-gpu.sh`, runs it, and leaves `tmp/run-on-gpu.{stdout,stderr}.txt` and
  `tmp/run-on-gpu.exit` behind; `tmp/gpu-loop.alive` ticks each iteration. It exists because a
  *window* needs the owner's session — a headless quorra device does not (ADR 0387), so anything
  that can be measured headless must be measured headless and never queued here.

  **The owner's rule, stated by them and absolute: use it only for measurements that require the
  real display or the real adapter. Under no circumstances for anything else — no file access, no
  installs, no fetches, no builds whose output matters, nothing that reaches outside the
  measurement.** The reason is worth understanding rather than obeying blindly: the script runs as
  `cl`, in `cl`'s session, with `cl`'s environment, so it can read and write everything this
  account deliberately cannot. That is a privilege boundary, and this loop is the one place where
  it is thin. A round that wants a file the agent user cannot read must say so in its report and
  leave it to the owner.

  What a queued script must do: terminate itself (`timeout N …`, and exit the viewer with
  `Escape` — `SIGTERM` skips the summary), never wait for input, use `./target/…` paths
  (`/home/AI` is unreadable by `cl`), and write only under `tmp/`. `xdotool` cannot reach a
  Wayland client, so force XWayland with `env -u WAYLAND_DISPLAY`.

- **Build directory**: `AI` builds into `/home/AI/cargo-target/pdf-viewer` via `~/.cargo/config.toml`,
  so the two users never fight over `target/`. Do not "fix" this. `pdfref` needs `--work-dir` for
  the same reason. A round that wants a build directory of its own — a worktree round does, so that
  parallel rounds do not queue on one build lock — asks for it with `--target-dir` and **not** with
  an exported `CARGO_TARGET_DIR`; the `sccache` note below says what the export costs.
- **A build script's `env!("CARGO_MANIFEST_DIR")` is baked at *its* compile time, and the shared
  build directory outlives a checkout.** `pdf-font`'s and `tools/conformance`'s build scripts read
  it, and a binary compiled from a worktree or a scratchpad copy that no longer exists fails with
  an absurd message naming a path under `/tmp` — "data/cmaps is readable: No such file or
  directory". It is not the tree. `touch` the build script's source and rebuild. Two rounds of the
  four-hundred-and-fifties lost time to it.
- **`sccache` is the `rustc-wrapper`, and `export CARGO_TARGET_DIR=…` is what makes it useless.**
  It is activated for user `AI` in `~/.cargo/config.toml` (`build.rustc-wrapper`, an absolute path
  to `~/.cargo/bin/sccache`, which is **not on `PATH`** — `which sccache` answers nothing while
  every build goes through it, ADR 0264 again). `/home/cl/projects/render-lib` inherits the same
  wrapper: its own `.cargo/config.toml` overrides only `build.target-dir`, and `cargo build -v`
  there prints the wrapper in front of `rustc`.

  **`sccache`'s Rust cache key includes every environment variable whose name begins with
  `CARGO_`.** So a round that exports its own `CARGO_TARGET_DIR` — one per worktree, which is how
  parallel rounds avoid sharing a build lock — gives itself a private cache namespace that nothing
  will ever read again. It is not the *path* that does this and not the changing source: the same
  build, the same warm cache, a fresh target directory named on the command line instead of in the
  environment, moves the hit rate from nothing to most of it. ADR 0344 has the A/B and the four
  reasons the rest of the compilation is uncacheable in principle.

  **So: name the target directory on the command line (`cargo … --target-dir <dir>`) or in a
  `.cargo/config.toml`, never in the environment.** Both are invisible to `sccache`; the export is
  not. This costs nothing and needs no agreement from anyone else's round.

  `sccache --show-stats` is the instrument and its *categories* are the answer, not its headline
  rate: `Cache hits (Rust)` against `Cache misses (Rust)`, and `Non-cacheable reasons` underneath —
  `crate-type` is every binary and every test harness, `multiple input files` is every
  workspace-member `clippy` check. `sccache --zero-stats` first, but only against a server of your
  own (`SCCACHE_DIR=… SCCACHE_SERVER_PORT=… sccache --start-server`): the default one is shared
  with every round running at the same time, and zeroing it destroys their measurement as well as
  yours.

  Two older cautions still hold. It caches *compilation*, which is what makes the stale-build-script
  hazard above likelier. And **it must not be allowed to make a measurement**: a round timing a
  build says which wrapper was in place, and a round measuring the program rather than the build is
  unaffected, because `sccache` touches compilation and nothing the binary does.

  **A third, written down because it is what this wrapper is *not*: it has nothing to do with
  `cargo miri` being slow here.** The six-hundred-and-fourteenth session spent an hour on a Miri
  run, blamed `sccache`, and was wrong twice over. `cargo-miri` sets `RUSTC_WRAPPER` to **itself**
  before invoking Cargo, so this machine's wrapper is never in front of the interpreter and
  `RUSTC_WRAPPER= cargo +nightly miri test …` changes nothing; and with it "cleared" anyway,
  `-p pdf-syntax` still ran past half an hour of CPU inside the *runner* phase — not compilation.
  **The discrepancy is closed, and it was two tests** (ADR 0463): the interpreter is four orders of
  magnitude slower than the processor, so a test whose *input* is large pays for every byte of it —
  an LZW bomb decoding to 7 MB, and a sweep of 1.8 million number lexes. Both now say what they do
  under Miri, in their own doc comments. The lesson that generalises is the older one two bullets
  up: a wrapper must not be allowed to make a measurement, and *neither must a hypothesis about
  one*. **The instrument that answered it was CI's own log** — `gh run view --job … --log`, whose
  per-test timestamps attribute the hour without running anything here.
- **`cargo-fuzz` needs `+nightly`** explicitly; `rust-toolchain.toml` pins stable 1.97.1
  deliberately. `cargo-deny` is in the agent's `~/.cargo/bin` — **and so is `cargo-fuzz`, which is
  not on `PATH`**, so `which cargo-fuzz` answers nothing and `cargo fuzz` fails with "no such
  subcommand". Sessions 425 and 426 read that as "cargo-fuzz is not installed here" and left a
  target unwritten on the strength of it; it has been there since 26 July. Prefix the run:
  `PATH=$HOME/.cargo/bin:$PATH cargo +nightly fuzz …`. **`which` answers a question about `PATH`,
  not a question about the disk** (ADR 0264).
- The Arlington model is a **submodule** pinned at `ba7d4d61`; `pdf-spec` will not build without
  `git submodule update --init`. **It is the only submodule a build needs.** `doc/pdf.js` is what
  the ratchets are measured over, so CI must have it; the four corpora under `doc/corpora/` are
  optional in the strong sense — no gate names one, and the tests that name a path inside one print
  that it is not checked out and pass. **Two of the four want a sparse clone rather than
  `git submodule update --init`**, which would take the whole upstream repository; the recipes are
  `doc/oracle-and-corpus.md` §2 and they are 73 MB and 12 MB instead.
- **`tmp/hayro` is a checkout of the whole hayro workspace**, with the project owner's fork as
  `origin` and the maintainer's as `upstream`. **The owner's standing offer is that a fix goes on a
  branch there, they push it and open the pull request, and this tree depends on the fork
  meanwhile** — so a defect in `hayro-jpeg2000` or any other member is a branch to write rather than
  a dependency to wait on. `doc/JPEG2000_FEEDBACK.md` §9 has the detail and the precedent. **This
  changes what a todo file may call blocked**: "waits on the decoder's API" is a statement about
  effort, not about access.

## The specifications, and the one command a fresh clone needs

**The specifications are in this tree encrypted, and that was not engineering.** The fourteen ISO
and PDF Association documents in `doc/` and their Markdown conversions under `doc/md/` were
**tracked in the clear, and the project owner is not licensed to redistribute them** — free to
obtain is not the same permission, and a repository carrying them passes them on to everyone who
clones it. In the three-hundred-and-eleventh session they left the tree, the index and **all 436
commits of the history**, and came back **encrypted** (ADR 0187): `doc/specifications.zip`, 37 MB,
ZipCrypto, all twenty-eight files, with `.gitignore` covering what `unzip` puts back. `git log
--all --name-only` finds no path under `doc/md/` and no `doc/*.pdf` in any commit, which is the
only check worth trusting on this. **This tree may be published**; nothing else here had to be
true first.

**Run this once in a fresh clone, and every gate and example in this tree works:**

```sh
unzip -P <password> doc/specifications.zip    # from the workspace root; ask the owner
```

**Every reference to the documents stays as it was**, decided by the owner in that session: four
tests and eleven measurement examples open `doc/ISO_32000-2_sponsored_EC3.pdf` or
`doc/PDF20_AN001-BPC.pdf` and fail loudly until you have, and `cargo test -p conformance` checks
no citation without `doc/md/ISO_32000-2_sponsored_EC3.md`. **CI is a developer like any other
here** and unpacks the archive from the `SPEC_ZIP_PASSWORD` repository secret before its tests;
a pull request from a fork gets no secret, and the step says so rather than failing obscurely.

