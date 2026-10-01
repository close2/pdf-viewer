# 1487 — A state section reads and never writes

Session 1326. Status: accepted. Code: `tools/conformance/src/bin/ledger.rs` (counts by default,
writes under `--write`), `tools/state.sh` (header, `ledger`, `comments`, `governing`, `batches`,
`drive`), `tools/main-checkout.py` (`git --no-optional-locks`). Tests:
`tools/conformance/tests/read_only.rs`, `tools/conformance/tests/owner_section.rs`. Prose:
`doc/todo/02` section 8 item 4, `doc/verify.md`, `doc/environment.md`.

## 1. The question

`CLAUDE.md` names `tools/state.sh` as the command that counts. After a merge the orchestrator ran
it in the main checkout and found `doc/conformance/ledger.toml` modified, and the next
fast-forward refused until the change was discarded. A counting command had written.

The cause was the `ledger` section, which ran `cargo run -p conformance --bin ledger` — the
ledger's *generator*. It re-reads the standard's clause index, keeps every row, adds a row for any
subclause that has none, and writes the whole file back in its canonical form. The form differs
from the file whenever a person writes a row's keys in another order: today §8.6.6.6 carries
`test` above `code`, so every run moved one line, and that was the "test lists reordered". The
printed counts were a by-product of a write.

## 2. The rule

**A `tools/state.sh` section reads and never writes** — not a tracked file, not an ignored one, not
a lock in a repository. Six rounds edit one worktree at once and the merge fast-forwards from it,
so a write nobody asked for is at best a refused merge and at worst a sibling's edit overwritten.

- **The generator writes only when asked.** `--bin ledger` without arguments builds the file it
  would write, prints the status counts of that file and whether the one on disk is already in
  that form (and from which line it is not), and writes nothing. `--bin ledger -- --write` is the
  generator, run by a round editing the ledger on a tree nobody else is editing. A separate binary
  was the other shape; one binary with the write behind a flag keeps every document that says
  "`--bin ledger` prints the counts" true and puts the one dangerous verb where it must be typed.
- **`git status` and `git diff` take `index.lock`** to refresh the stat cache, unless run with
  `--no-optional-locks`. `tools/main-checkout.py` ran both in the owner's checkout and in the
  worktree; it passes the flag now.
- **Every `tools/*.py` the script runs carries `PYTHONDONTWRITEBYTECODE=1`** (trap 69). A script
  run directly writes no bytecode for itself, so `comments` and `governing` wrote nothing; the
  flag is the uniform rule so that the day one imports a sibling, nothing changes.

## 3. How it is held

`tests/read_only.rs` reads the script by construction rather than by running it: running it would
put `cargo` inside `cargo test` on one build directory, and with six rounds editing the tree a
before-and-after comparison measures the siblings. The script is lexed (comments, here-documents
and quoted text set aside, a `$( … )` inside double quotes kept as code), and each command is
asked five things: a redirection writes only to `/dev/null`, a descriptor or a `mktemp` file; no
writing command (`rm` of anything but a `mktemp` file, `cp`, `mv`, `touch`, `tee`, `mkdir`,
`ln`, `sed -i`, a `git` subcommand that changes a repository); `git status`/`diff` carry
`--no-optional-locks`, in the script and in each `tools/*.py` it runs; a `tools/*.py` runs with
`PYTHONDONTWRITEBYTECODE=1`; a `conformance` binary whose source writes accepts `--write` and is
run without it. A planted script calibrates each rule and the shapes it passes.

The construction was checked once by measurement. Each `quick` section was run under
`strace -f -e trace=openat,creat,rename*,unlink*,mkdir*,truncate,link,symlink`, keeping every
successful write outside the build directory and `scratchpad/`:

| section | wrote |
|---|---|
| `ledger` (before) | `doc/conformance/ledger.toml`, the whole file |
| `main-checkout` (before) | `.git/index.lock` in the main checkout and `.git/worktrees/<name>/index.lock` |
| `conformance` | only under `std::env::temp_dir()`: `batch.rs`'s throwaway repositories |
| every other `quick` section, and both of the above after | nothing |

What the test cannot see is said in its header: a `cargo test` a section runs writes what its
test binary writes, and a binary of another package is read only through the words of rule 2. The
corpus walks outside `quick` write what their gates write and are the merge's.

## 4. The owner's section, held to its printer

`doc/environment.md`'s *After a merge* says per line what `tools/main-checkout.py` prints, "in the
order it prints them" (ADR 1440), and the script's docstring said a new line owes an entry. Two
lists kept by hand; `tests/owner_section.rs` now derives both — the kinds from the strings each
function the script's `main` calls returns, prints or appends, keyed by their words before the
first `:` or `,`, in call order; the entries from the fenced block's comment lines that open with
a backtick-quoted line — and asserts one list in one order, and that each indented sub-line the
script prints (`read, removable:`, `unread:`, `owed:`, `no Repository:/Base: preamble:`) is named
in the section. Its first run found the last of those named nowhere; the patches entry now says
what it means.

## 5. Two read-only lines added beside the rule

- **`tools/state.sh batches`**: per batch commit whose body has a `Round durations` paragraph
  (ADR 1476), the gates figure `N of M green, T s of gate wall time` as the message wrote it, and
  the sum of that paragraph's figures written `<n> s` with how many it summed — so a figure
  written another way shows as missing from the count rather than silently left out. The sum is
  of the orchestrator's notified durations, not of a gate's figures. `doc/todo/02` section 8 item
  4 asks the orchestrator to write each round's wall time that way.
- **`tools/state.sh drive`**: the verdict column of the newest `results.tsv` under `scratchpad/`
  (or `DRIVE_RESULTS`), counted as works, wrong, not offered and to look at (`manual`), with any
  other verdict named. No copy goes under `doc/`: a counted fact is printed, not written. The file
  is `tools/drive-windows.sh`'s own, at its default `--out` (`scratchpad/drive-windows/`) or under
  a round's `scratchpad/r<session>/`, and dies with the worktree, which is right for a count the
  next run replaces.
