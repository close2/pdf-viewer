# 1313 — A batch is committed by name, and a worktree with work in it is never closed

Status: accepted and **built**. Session 1238.
Amends: the merge loop of `doc/todo/02` section 8 (steps 5 and 8).
Context: `tools/batch.sh`, `tools/conformance/tests/batch.rs`, `doc/todo/02-every-round.md`.

## 1. The incident

The merge of batch thirty-five (sessions 1227–1232) ran one line: `git add $(cat list) && … ;
git commit … && git merge --ff-only … && tools/batch.sh close`. One path in the list was a file
already staged as deleted. It matches nothing on disk, so `git add` refused the pathspec — and
with it the whole list. The `;` after it let `git commit` run on an index that held that one
deletion; the fast-forward took the commit; and `close`, which removes the worktree with
`git worktree remove --force`, deleted the tree with 128 uncommitted files in it. Nothing in the
chain was wrong alone. Each command succeeded on the state the one before had left, and the one
that failed was the one whose exit status nobody read.

The tree was rebuilt by replaying the six rounds' transcripts — every `Write`, `Edit` and
file-mutating shell command, in timestamp order — and each round then verified its own files
read-only. The real commit is `058e77dd`. The method is `doc/todo/02` section 8 step 8.

## 2. The decision

- **`close` refuses a worktree holding anything uncommitted outside `scratchpad/`**, lists what
  is there, and removes nothing. It offers no `--force`, and a spelling of one is refused rather
  than read as a branch name. The fix for the refusal is to commit the work or to move it.
  Discarding it blind is not an answer this command gives, and **a later round must not add one**:
  the only state in which `close` is ever wrong to refuse is one where somebody has already
  decided the work is worthless, and that decision is `rm`'s, typed by a person who has read the
  list, not a flag on the command that runs last in a merge.
- It also refuses a branch name the worktree is not on, since every later check reads the name,
  and a failing `rev-list` now refuses instead of counting as "no commits main lacks".
- **`commit <message-file>` stages the population by name** — every path `git status` reports
  outside `scratchpad/`, which is the population `check` reads — with `git add -A` over those
  explicit paths. A deletion already staged is counted and not named to `git add`, which is the
  incident's own pathspec. It prints the population's count against the index's, and refuses to
  commit when the two sets differ, when a staged path is under `scratchpad/`, or when a submodule
  is staged as a blob. After the commit it prints what is still uncommitted, which must be 0.
- **It never fast-forwards and never closes.** It prints the next command instead. Section 8
  step 5 now states that the commit, the fast-forward and the close are three commands, run
  separately, each read before the next.

## 3. What holds it

`cargo test -p conformance --test batch` runs a copy of the script against a throwaway repository
(`BATCH_WORKTREE` moves the worktree; the script's root follows where the copy lives): a dirty
worktree is refused with its paths named and nothing removed, `--force` and a wrong branch are
refused, the incident's shape — a staged deletion beside new and modified files and a
`scratchpad/` — commits whole and without `scratchpad/`, and `close` refuses until `main` has the
commit. Against the committed script of `058e77dd`, the same dirty worktree is closed and its file
is gone; against this one it is refused and the file is there.
