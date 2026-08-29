# 0752 — The populations our own instruments assume

Status: accepted.
Context: `tools/round.sh`'s build-script check, `tools/worktree.sh list`, and `tools/state.sh
disk`. Three housekeeping instruments, one question asked of each: *is the set of things you look
at derived from the tree, or written down beside it?*

## Why that was the round's question

The round before this one closed an orphaned build directory and left a note: `tools/worktree.sh
list` globs `pdfv-r*`, so two directories named otherwise were invisible to it for hundreds of
rounds — and *the interesting half is whether anything else the tools glob for has the same
shape*. That is the whole subject. A glob, a hard-coded list and a hard-coded line range are the
same construction: a claim about the tree, standing beside the tree, with nothing comparing them.

This project already knows the argument and has written it down once, in the file that turned out
to carry two more instances of it. `tools/worktree.sh`'s gitlink guard used to be a hand-written
list of two corpora while four paths were being symlinked; the comment that replaced it says the
population is derived from the index because "[a] list written by hand goes stale the next time
something is linked, which is exactly how this one did". The lesson had not travelled two
functions down the same file.

## What each instrument was asking, and what it was actually asking

### 1. `tools/round.sh`, check 2 — the build script baked against a checkout that is gone

The mechanism is real and this tree has paid for it: `env!("CARGO_MANIFEST_DIR")` is expanded when
a build script is *compiled*, so the path is baked into the binary, and the shared build directory
outlives the checkout it names. The check greps compiled build-script binaries for such a path and
asserts the directory still exists.

Its population was the literal list `'pdf-font' 'conformance'`, written when the check was, four
hundred and thirty-five commits before this round. Both halves were wrong:

- **`tools/conformance` has never had a build script.** Ask `git log --all` for a `build.rs` under
  that directory and it prints nothing; the crate's `Cargo.toml` takes one dependency and declares
  no `build`. So half of every run of this check has looked for a file that does not exist, found
  nothing, and contributed a `✓`. (The path is not written out here on purpose: `--bin pointers`
  would read it as a pointer to a missing file, which is the one thing that sentence is *about*.)
- **`crates/pdf-sandbox/build.rs` was never asked**, and it is a *worse* instance than `pdf-font`.
  It bakes the manifest path with `env!` and then walks a directory under it, panicking with
  `"{} is readable: {error}"` if the checkout is gone — and what it computes is the identity the
  worker's greeting compares (ADR 0458). The build directory on this machine holds a `pdf-sandbox`
  build script naming `.claude/worktrees/r656/crates/pdf-sandbox`, which is gone. The old check saw
  six superseded stale paths there; the derived one sees seven.

**The discriminator has to come off the source rather than off the artefact**, and that is the part
worth having. `crates/pdf-spec/build.rs` reads the *same variable* through
`std::env::var_os("CARGO_MANIFEST_DIR")` — read when cargo **runs** the script, so cargo supplies
the live value and it cannot go stale — and yet its compiled binary carries the path anyway, in
debug info. Grepping a binary for a path finds strings the program will never read. Adding "every
crate with a build script" to the population would have produced six false positives on this disk;
adding "every build script that expands `env!`" produces the two that can fail.

So the population is now derived: every tracked `crates/*/build.rs` and `tools/*/build.rs` whose
source contains `env!("CARGO_MANIFEST_DIR")`, with the package name read out of the crate's own
manifest and the grep pattern built from the crate's own path — one list instead of two that had to
agree. **An empty population fails rather than passing**, because that is exactly the state the old
check spent half its life in.

### 2. `tools/worktree.sh list` — every build directory, or the ones this script names?

The loop globbed `"$builds"/pdfv-r*`, which is the naming `open_one` invents, and printed each as
`live` or `ORPHANED — its worktree is gone`. It could therefore only ever report a directory this
script made. When the widening was run here, the directories the glob could not name were most of
the root's size — the figure is `tools/worktree.sh list`'s now rather than a sentence's — and the
two the previous round removed by hand, `pdfv-759-before` and `pdfv-759-after`, had been inside
that invisible set for hundreds of rounds.

The four classes a directory now falls into are stated rather than assumed: the main checkout's
(*derived*, from `cargo metadata` in the main tree, because nothing here chose its name); a live
round's; an orphaned round's; and one no checkout here names, which this script must report and
must not judge — `quorra`, `hayro` and the probe directories are other work's and deleting them is
not this command's business.

And it totals, because the total is what the rule needs. `du -h` picks a unit per directory and a
column of mixed units cannot be added, so the addition is done in `du`'s kilobytes and only the
printing picks a unit — one walk of the root rather than two.

### 3. `tools/state.sh disk` — the round's directory, under a threshold about the root

`doc/todo/02` §5a says to sweep "when it passes a hundred gigabytes", and `disk` was the command it
pointed at. `disk` reports the round's own `target-dir`, asked for rather than written down — which
is correct and deliberate, and is trap 15's fix: the literal main-tree path it used to carry
reported a *neighbour's* directory from inside every worktree.

But from a worktree the round's own directory is a few hundred megabytes and the root holding it is
over a hundred and fifty gigabytes. Both numbers are true and only one of them answers §5a. So the
existing line stays and the root is printed beside it — under three conditions, because the root
being the parent of the target directory is this machine's arrangement rather than a derivation: it
must not be the repository (in an ordinary clone the parent of `target/` is the source tree and its
size says nothing about builds), must not be `.`, and must hold more than the one directory.

## The alternative that was rejected

Widening the glob by one character — `pdfv-*`, or `*` — and leaving the rest. It closes the
directories this round found and none of the class: the next probe directory with an unrelated name
is invisible again, and the listing still cannot say whose a directory is. The point is not that
the pattern was one character too narrow; it is that a listing's population was the writer's own
naming convention while its heading promised the disk.

## What binds after this

`doc/traps/instruments-and-reports.md` trap 25, whose sentence is the one that generalises:
**a narrow population and a clean tree produce the same output.** That is trap 23's failure with
the instrument's input wrong instead of its scope, and trap 24's with a list instead of a corpus,
and it is why none of these three was ever going to be found by reading the instrument's output.

No gate is added, deliberately. A gate over the first population would restate the derivation it
now performs — `tools/conformance/tests/workspaces.rs` exists because §2's command block is prose
and cannot derive anything, and none of these three is prose any more. What is left to keep is the
habit, and the trap is where a habit goes.

## Calibration

Trap 13, on the check that gained a population, in a scratch build directory with the defect
planted and both versions run over it:

| planted | old population | derived population |
|---|---|---|
| `pdf-sandbox`'s newest build script names a gone checkout | silent — a `✓` | fails, naming the path |
| `pdf-font`'s newest build script names a gone checkout | fails, naming the path | fails, naming the path |
| both name live checkouts | silent | silent |

The second row is the half that matters as much as the first: the derived population does not lose
the case the written-down one held.
