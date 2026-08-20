# 0450 — The gate that only ran somewhere else

Status: accepted.
Session: the six-hundred-and-fourteenth.
Supersedes nothing; amends `doc/todo/02` §2's lint line, `doc/verify.md`'s cross-target block and
`doc/todo/52`'s skip; extends ADRs 0194 and 0246; the failing pipeline is trap 10's second half
(`doc/traps/instruments-and-reports.md`) with the reporter on somebody else's machine.

## What happened

The GitHub Actions pipeline had failed on every push for five runs and a week, and the project
owner found out rather than the project. Five of seven jobs were red in the last of them. Not one
of the five was visible from this machine, and that — not any of the five defects — is the finding.

| job | what it actually was | kind |
|---|---|---|
| `check` | `viewer-qt`'s `cpp/window.cpp` uses `QPalette::Accent`, which is Qt 6.6's; the runner's `qt6-base-dev` is older, so the build script failed and `clippy` never ran | a defect here — a version requirement made silently |
| `test` | the same build script, in the same graph, before a single test ran | the same defect |
| `build (macOS)` | `unused variable: wake` in `viewer-accessibility/src/bridge.rs`, an error under the workflow's `RUSTFLAGS: -D warnings` | a defect here — a cross-target check nobody ran |
| `build (Windows)` | the same line | the same defect |
| `nightly` (Miri) | `crossbeam-epoch` 0.9.20 fails Stacked Borrows under `rayon`, reached from `pdf-render`'s divided reduction | a dependency's unsafe, uncovered by a skip that named a different one |
| `deny` | passed | — |

**None of the five was a CI configuration that had drifted.** No action version, no cache key, no
toolchain, no `SPEC_ZIP_PASSWORD` step: every one of them was this tree, and every one was
reproducible here once somebody ran the command CI runs.

## The finding: three gates, each weaker here than there

The five failures are three instruments, and each was weaker on this machine than in the workflow
in a *different* way. That is the pattern worth keeping.

### 1. The lint gate, weaker by a flag

`doc/todo/02` §2 said `cargo clippy --workspace --all-targets`. The workflow says the same thing
with `RUSTFLAGS: -D warnings` in its environment, and the workspace's lint levels are `warn` so
that an ordinary build stays usable. So the local gate could be silent on a warning that fails the
push — `CLAUDE.md` principle 1's "warnings are errors in CI" was a statement about one machine,
and the machine every round works on was the other one.

`doc/verify.md` had already worked this out for the *cross-target* checks and wrote it down in as
many words: "**`RUSTFLAGS` is not optional** … a cross-target check without it is a different build
from the one that gates a push", after three dead constants got through exactly that gap (ADR
0194). The same sentence was never carried back to the line every round runs. **§2's clippy line
now carries the flag**, and the note beside it says why.

What it does *not* buy is any of the five failures above: with the flag set, this tree's host-target
lint run is silent, before and after this round's edits. The gap was real and the warning that fell
through it was on another target — which is the second instrument.

One thing found beside it and worth the sentence: **`tools/state.sh` runs neither of §2's first two
lines.** That is right for what the script is — its subject is figures and a silent lint has none —
but §2 said it "runs the same sequence", so a round that reached for the script had not linted or
checked formatting at all and had no way to know. The sentence now says which two it leaves.

### 2. The cross-target checks, weaker by not being run

`doc/verify.md`'s Windows `-p viewer-ui` line, with the flag it already carried, reproduces the
`wake` error in fifteen seconds. It was never run, because it is not in `doc/todo/02` §2's change →
gate map and `viewer-ui`'s row in that map says "no corpus gate — the core, which builds and tests
them". The core builds them **for this host**, and `viewer-accessibility::Bridge::new` takes a waker
that only the Linux build has an adapter to give.

The fix is not to enlarge the map, which would make every round pay four cross-compiles. It is that
the block in `doc/verify.md` now says what that line stands in for — CI's `platforms` job — and
gains its macOS twin, so that a failure names its platform instead of implying it.

The code fix is one line and is *not* an underscore: the parameter stays in the signature on every
platform so a host writes one call, and `drop(wake)` releases what it captured at the point the
Linux build hands it to `Actions`. `_wake` would have said the argument is unused, which is a
different and false claim.

### 3. The Miri skip, weaker by being a string in a workflow

The Miri line read `-- --skip flate`, standing in for "the tests that drive `zlib-rs`'s unsound
deallocation" (`doc/todo/52`). A name-substring filter was wrong in both directions:

- it took a **third** test with it — `an_inflate_never_buys_a_buffer_past_the_bound` contains
  `flate` inside `inflate` — while `doc/todo/52` said two, so a test about a decompression bomb's
  buffer was excluded by an accident of spelling that neither end could see;
- and it could only ever exclude what somebody had remembered to name *there*. When a **second**
  dependency's unsafe appeared — `crossbeam-epoch`, reached through `rayon` from `pdf-render`'s
  divided reduction, added when that reduction was divided — the job simply went red, and the file
  holding the exclusion had nothing to say about it.

**Each test that must not run under the interpreter now declines by itself**, naming its dependency
and its aliasing rule, and the workflow line carries no `--skip` at all. This is `doc/todo/02` §2's
own rule for a gate binary's tests, applied one level out: *an invocation can be copied without its
guard, and a test cannot be run without itself.*

Two things this deliberately does not do. It does not switch Miri to Tree Borrows: `zlib-rs` fails
both models (`doc/todo/52` has both messages), so the looser discipline would buy nothing and cost
the stricter one everywhere else. And it does not treat `crossbeam-epoch` as this project's debt to
report the way `zlib-rs` is — the retag is inside a work-stealing deque three crates away, the
project did not choose `crossbeam-epoch`, and `rayon` is not optional here.

## The Qt version, which is a decision rather than a fix

`QPalette::Accent` is Qt 6.6's. Asking for it unconditionally is not a portable request but a
version requirement, and `viewer-qt` made one without declaring it — which is why the *whole
workspace's* `clippy` and `test` were red rather than one host's.

Two ways out. Install a newer Qt on the runner: a third-party action, a download, and a version
this project would then have to keep chosen — for one enumerator. Or ask for the enumerator where
it exists. **The second**, in `accentOf`, with `QPalette::Highlight` standing in below 6.6.

The substitute is not arbitrary and the reason it is acceptable is the reason ADR 0246 asked for
the accent at all: `Highlight` is the same idea one step less specific — the desktop's selection
brush, which on most colour schemes *is* the accent — so an older Qt draws §12.5.1's focus ring in
a colour that is still the platform's and never one invented here. What matters more is that the
substitution is **not silent**: the palette role's own name travels beside the colour, and the
sentence `MainWindow` prints on start-up names whichever one this build asked for. ADR 0246's
evidence for "chrome in the platform's colours" is that sentence being checkable, and a build that
printed `QPalette::Accent` while using the highlight would have quietly cost it.

The `#else` branch cannot be compiled by this machine's Qt 6.11 — the preprocessor takes the other
one — so it was compiled on its own, against Qt 6 headers with `-Wall -Wextra`, before being
believed.

## Why nobody noticed, and the cheap half of the answer

A round works in a worktree branched off `main`; its gates are a statement about that worktree; the
merge is a round of its own and pushes. Nothing in this project has ever *asked* what happened to a
push. `tools/round.sh` is where a round finds out what previous rounds got wrong, and it now asks
GitHub for the last run on `main`.

It is a **report and not a gate**, which is trap 10's distinction and matters here: it depends on a
network and a token, so it can print three things rather than two — green, red with the command
that shows why, or *not asked*. A silence is never rendered as green. The token is read through
git's common directory rather than a relative climb, because a worktree is an arbitrary distance
from it.

`workflow_dispatch` is added to the workflow's triggers for the other half: today the first time CI
sees a round's work is the push that has already landed. GitHub offers that trigger only for a
workflow on the default branch, so it becomes usable one merge from now — which is honest about
what it is rather than a fix this round can demonstrate.

## A sixth failure, found by reading the runs before the last one

The run one week earlier has a `test` job that ran **six hours and one minute** and was cancelled at
the platform's ceiling. It is not a hanging test and it is not this tree: the log stops inside
`apt-get update`, on the runner's own Azure mirrors, four `Ign:` retries into a package list. An
infrastructure stall, not a defect — but GitHub's default is to let a job run to its six-hour
maximum, so it cost six hours of runner time and, worse, produced a failure whose shape says
nothing about its cause.

Every job now carries `timeout-minutes`, generous against what each actually takes. A ceiling is not
a fix for a stall; it is what makes one *legible*, which is the same argument as everything else in
this ADR.

## What this round could not do, and it is worth writing down

**None of this was proven by a green run**, and the reason is a third instrument being weaker than
it looks. The token this machine holds is read-only for contents: `git push` answers 403
("Permission to close2/pdf-viewer.git denied"), and so does `POST /repos/…/git/refs`, so a branch
cannot be put on the remote and a pull request cannot be opened for one that is not there. The
workflow's triggers are `push` to `main` and `pull_request`, and `workflow_dispatch` — added
here — is offered by GitHub only for a workflow already on the default branch. So there is no
route from this account to a run, and the merge that lands this file is the first thing that can
produce one.

That is a fact about the account rather than an argument, and it makes the local reproduction carry
the whole weight. Where each of the three stands, exactly:

- **the two platform jobs** — reproduced and then cleared here, by the `-D warnings` cross-target
  checks that are their local equivalent, on both targets;
- **the Qt enumerator** — *not* reproducible on this machine, whose Qt is too new to lack it, so the
  `#else` branch was compiled on its own against Qt 6 headers under `-Wall -Wextra` rather than
  believed. This is the one place where the merge's run is the first real evidence;
- **Miri** — run here, on the crate that was failing. `pdf-render`'s library tests come back
  **168 passed, 0 failed, 1 ignored** — the one being the divided reduction — and the interpreter
  reaches the end where before it aborted inside `crossbeam-epoch`. `pdf-syntax`'s run was still
  going when the round closed, but its declinations were already visible in the output rather than
  inferred: the harness prints `an_inflate_never_buys_a_buffer_past_the_bound ... ignored, zlib-rs's
  deallocation, not this tree's`, and a reason a person reads in the run is the whole argument for
  putting it on the test instead of in a workflow.

  **`pdf-syntax` under Miri is slow enough here to look broken, and the round's first explanation
  for that was wrong.** Three runs:

  | run | what | result |
  |---|---|---|
  | both crates, `sccache` as `rustc-wrapper` | killed after 35 min | reached `pdf-syntax`'s tests |
  | `-p pdf-render`, `RUSTC_WRAPPER=` cleared | **3 min 54 s** end to end | 168 passed, 1 ignored; 174 s of interpretation |
  | `-p pdf-syntax`, `RUSTC_WRAPPER=` cleared | **still interpreting past 33 min of CPU** | — |

  The middle row tempted this ADR into blaming `sccache`, and the third row refutes it: with the
  wrapper cleared the same crate is still slower than everything. It is also not compilation —
  `MIRI_CWD` is set in that process's environment, which is `cargo-miri`'s *runner* phase — and
  clearing `RUSTC_WRAPPER` turns out to be a no-op anyway, because `cargo-miri` sets that variable
  to *itself* before invoking Cargo. **So the wrapper was never in front of the interpreter, and
  the first explanation was wrong in its mechanism as well as its conclusion.**

  What is left is a real discrepancy with no diagnosis: CI did both crates in 2 min 39 s the last
  time it was green, and one of them has not finished here in ten times that. It is worth a round's
  attention and it was not this one's — the question here was whether the *declinations* work, and
  `pdf-render`'s run answers that on the crate that was failing.

**A round that merges this owes `doc/todo/02` §2 on `main` and a look at the run it triggers.**

## Consequences

- Every round's lint run is now the run that gates a push, and pays one extra check pass the first
  time.
- A round that touches `viewer-accessibility`, `viewer-ui` or anything under them owes
  `doc/verify.md`'s four `-p viewer-ui` / `-p viewer-ffi` lines; the map still does not force them,
  and the block now says which CI job they stand in for so a round can judge.
- `viewer-qt` states a Qt floor it can actually meet, and says out loud which colour it got.
- The Miri line is exclusion-free, and a new dependency whose unsafe fails an aliasing model will
  fail *visibly*, on the test that reaches it, rather than being absorbed by a filter.
- `tools/round.sh` can fail on something no command in this tree produces. That is new and is the
  point.
