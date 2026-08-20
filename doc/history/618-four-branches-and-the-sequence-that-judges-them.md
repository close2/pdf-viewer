# 618 — Four branches, and the sequence that judges them

The first merge round this project has had, because it is the first time four rounds ran at once.
`doc/todo/02` §2 has carried the rule since the 455–484 block paid for it — *green in a worktree
establishes nothing about `main`* — and until now it was a rule about a rare event. It is now the
ordinary case, so this file records what a merge round is, what this one found, and the two hazards
that are the *worktrees'* rather than the rounds'.

## What was merged

`round-614`, `round-615`, `round-616` and `round-617`, all branched from `0b9a18b3`, eight commits.
**Four clean merges, no conflicts** — which is worth writing down because it was not luck: each
round was told which files were shared (`doc/conformance/ledger.toml`, `doc/todo/README.md`,
`doc/HANDOVER.md`, `doc/verify.md`) and to keep its edits inside the rows and lines it was actually
reading. 615 and 616 both changed `shading.rs`, `pattern.rs` and `tests/shadings.rs` and still did
not collide, because one was adding a *bound* to a mesh's painting surface and the other a *report*
for an entry nobody read.

## The sequence, run whole on `main` after the merge

| | |
|---|---|
| `fmt --check` | clean |
| `clippy --workspace --all-targets` | silent, **plain and under `RUSTFLAGS="-D warnings"`** — 614's correction, applied here first |
| `nextest --workspace` | 2283 passed, 16 skipped |
| doctests, `-p conformance` | clean |
| corpus | 974 documents, **68 incomplete** |
| oracle | 1794 pages — 907 agrees, 66 contradicted, 786 ambiguous |
| `render-quorra` | 957 pages — 932 agree, 23 differ, 2 refused, 17 not comparable |
| text extraction | 10969/11163 words in bounds (98.26%), 486 of 508 documents |
| selection census | 1000/1011 dragged words (98.91%) over 453 documents |
| accessibility census | 0 defects |
| dates, XMP, JPEG 2000 | clean |

**The incomplete count rose by two and that is not a regression**, which is trap 5's own sentence:
616 made Table 77's `/Background` loud, and the two documents that state it now say so. A count that
rises because a silence became a report is the instrument working. The oracle's three verdict
totals are identical to what each branch measured alone, so no two rounds' pixels interfered.

## What a merge round is for, demonstrated

Nothing here failed — but the round is not therefore a formality, and the evidence is in the run
above rather than in an argument. **Each branch's own `clippy` run was the weaker one**: 614 found
that §2's line carried no `RUSTFLAGS` where CI sets `-D warnings`, and fixed §2 — so this is the
first sequence in the project's history run under the flag the rule always stated. Three of the four
branches were linted before that fix existed. A merge round is where a correction to the gates
reaches the work that was done without it.

## Two hazards that belong to the worktrees, not to the rounds

Both were found the hard way and both are now in `doc/environment.md`, written by 614.

- **`git add -A` and `git add -u` destroy the submodule gitlinks here.** A fresh worktree carries
  neither the gitignored specifications nor a submodule checkout — `pdf-spec` will not build — so
  `doc/md`, `doc/*.pdf`, `doc/pdf.js`, `doc/corpora/*` and `doc/arlington-pdf-model` were symlinked
  into the main worktree. Git then records those symlinks as blobs and the six gitlinks are gone,
  producing a checkout CI cannot build. 614 hit it twice in its own branch and
  `every_declared_submodule_is_still_tracked_as_one` caught it both times; 616's branch had it and
  was repaired before the merge. **The gate is the reason this cost minutes rather than a session.**
- **A shell's working directory is not a guarantee.** 614's moved into 616's worktree with no `cd`
  and an amend landed on the neighbouring branch. Nothing was lost — the amended commit's tree,
  parent, message and author date were identical, only the committer timestamp moved — but the rule
  it earned is unconditional: **carry `-C /…/worktrees/rNNN` on every `git` command in a parallel
  round.**

The older rule stands and is now sharper than ever: **`git stash` is shared between worktrees**, and
with four rounds running at once a stash is a neighbour's to take. None of these four used one.

## What is owed

The binaries were rebuilt and installed (§5), because four branches' worth of change reached
`pdf-model` and `pdf-render` and what a person runs should be what the tree says.

**CI is still unverified.** 614 fixed five real defects in this tree that the pipeline had been
failing on for at least five pushes, and could not watch a run go green: the token in the tree is
read-only for contents, so no branch reaches the remote and no pull request can be opened. The first
run that can judge 614's work is whoever next pushes `main`.
