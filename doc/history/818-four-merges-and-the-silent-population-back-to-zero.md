# 818 — Four merges, the silent population back to zero, and a seeder that met a protocol change it had not seen

Date: 2026-08-29. A merge round: `main` at `22e1feef` (813's result), four parallel rounds
integrated in round order, the whole of §2 run on the merged tree, §5's eight artefacts rebuilt and
installed, §4's sweeps taken against a pre-merge baseline in a checkout of its own, and every
worktree closed with its build directory. No feature work, and this round decided nothing that
needs an ADR.

**This round did not push, and `main` was pushed anyway — by somebody else, from this working
copy.** When the round started, `git branch -v` read `[ahead 96]`. After the four merges landed at
23:47 and while the gate sequence was running, `refs/remotes/origin/main` moved to `a292a5e6` — this
round's fourth merge commit — with the reflog reason `update by push`, at 00:15:45. The agent user
has no push credential at all (`git ls-remote` answers *Permission denied (publickey)*), so the
command was the owner's, run here. Nothing about the tree is wrong and nothing was lost; what
changes is one line of the owed list below, because CI now has this batch and its verdict is
readable rather than pending. Recorded because a round that reports "not pushed" while the ref
moved under it has told the next round something false.

`doc/rfc/` was not touched.

## The merges

| | branch | tip | merge commit | what it carried |
|---|---|---|---|---|
| 814 | `round-814` | `d33c7e32` | `a3d5a002` | `doc/todo/56` and its own history file — documents only, no source, **no ADR** |
| 815 | `round-815` | `7fd7b7f5` (4 commits) | `a792b9d1` | `article.rs`, `variable_text.rs`, ADR 0746, two ledger rows, `doc/errata-read.md`, `doc/todo/01` |
| 816 | `round-816` | `83b0e7ec` | `519b9ceb` | ADR 0747, `doc/verify.md`, `doc/traps/instruments-and-reports.md`, `fuzz/seed_confined_wire.py`, `tools/fuzz.sh` — no Rust |
| 817 | `round-817` | `3ca6b013` | `a292a5e6` | `structure.rs`, `accessibility.rs`, `tree.rs`, `protocol.rs`, a new example, one ledger row, ADR 0748 |

**Four clean merges, no conflict resolved by hand.** `ort` auto-merged the two files two branches
touched — `doc/conformance/ledger.toml` (815's §7.9.5 and §12.4.3 against 817's §14.8.5.5) and
`doc/errata-read.md` — with no hunk in common.

**814 branched from `7619c4ab`**, an intermediate commit of merge round 813, rather than from
`22e1feef`. Expected, and not rebased: the merge is a three-way against a base that is an ancestor
of `main` either way.

**Gitlinks.** `git ls-files -s doc/pdf.js doc/arlington-pdf-model` read `160000` before every
commit, and `conformance::every_declared_submodule_is_still_tracked_as_one` passes on the merged
tree. Every stage was by explicit path; no `git add -A`, no `-u`, no `stash`.

## `doc/errata-read.md`, the one contact the briefing predicted, resolved by argument

Both 815 and 817 wrote to this file and `ort` merged them without asking. A merge that produces no
conflict marker has still not shown that the two sentences agree, so both halves were read back out
of the merged file rather than trusted:

- **815's half is 144 added lines** — the sixteenth use of the errata ranking, its own section, plus
  the argument for weighing `pdf-association/pdf-issues` as a source and declining it. Every one of
  the 144 lines is present in the merged file, checked line by line rather than by diffstat.
- **817's half is one line rewritten** — the §14.8.5.5 row of the *fifteenth* use's table, whose
  verdict column said the row "went to `silent`" and now says that 817 read both entries and moved
  it to `implemented` (ADR 0748). Present.

**They do not contend, and the reason is worth writing down**: 815's new section names the
fifteenth use's erratum exactly once, as a *calibration* — it checked that pdf-issues numbers the
same issues this collection does by looking up an erratum this file already carried a verdict for —
and says nothing about that row's status. 817's edit is a status correction inside the earlier
section. One is the ranking's record of a new use; the other is a fact about a row an older use
found. Both stand, and the file reads in order.

## The seeder met a protocol change from another branch, and did not break

This is the merge rule earning its place, and it is the one thing in this batch no worktree could
have seen.

816 rewrote `fuzz/seed_confined_wire.py` so that its population is **derived** from
`viewer-confined/src/protocol.rs`'s `query_kind` rather than written down — its old sentence said
25 of 29 questions and by the time anyone acted on it there were seven missing and 32 carried. 817
then changed `protocol.rs`. On the merged tree those two meet for the first time.

They meet safely, and for a reason rather than by luck: 817 added two **fields** to
`AccessibilityNode` (`continues_a_list`, `continued_from`), not a `Query` discriminant, so
`query_kind`'s population is unchanged — and the seeder hand-writes only the *question* bytes and
keeps whatever the worker writes back, so an answer that grew two fields passes through it
untouched. Run on the merged tree against the freshly installed `target/pdf-view-worker` and two
documents, into a scratch directory rather than the shared corpus, it writes 68 seeds and exits 0.

Had 817 added a question instead, 816's script would have refused and named it, which is the whole
of what that change bought. Recorded because the *next* protocol change may be the other kind.

## The ledger's `silent` population is back to zero

811 wrote the ledger's first `silent` row and 813 recorded `silent 1`. 817 read both of Table 382's
PDF 2.0 entries, implemented them, and moved the row. On the merged tree
`cargo test -p conformance` prints the status breakdown with **no `silent` line at all** — the
checker lists only populated statuses — and the six that remain sum to the row count exactly:

```
conformance ledger: 875 subclauses
  implemented 447   partial 223   reported 17
  inapplicable 67   writer-side 8   out-of-scope 113
```

447 + 223 + 17 + 67 + 8 + 113 = 875. **0 rows are `unreviewed`**, in every one of clauses 7 through
14. `--bin ledger` against the baseline is the same statement from the other side: `implemented`
446 → 447 and the line `silent 1` gone.

The new census line 817 added prints beside it: **`§14.8.5.5's lists that continue an earlier one:
0 (0 with the predecessor on the same page)`**. Zero corpus witnesses, reported plainly, which is
what 817 said it would be. A capability with no witness is not a capability that failed.

## ADR verification, and the 0745 gap

| number | branch | present |
|---|---|---|
| 0745 | — | **absent, deliberately** |
| 0746 | 815 | yes |
| 0747 | 816 | yes |
| 0748 | 817 | yes |

**0745 is a permanent gap and both sides of it are in writing.** 814's history file says *"ADR:
**none.** A round that records the owner's decision rather than making one should not take an ADR
number"* — the source question in `doc/todo/56` was settled by the owner, not by the round. 815 had
already reserved two above the tip on the assumption that 814 would take 0745, and says so in ADR
0746's own second line: *"0745 is a sibling round's. This number was taken two above the tip on that
reservation."* So the gap is the parallel-rounds numbering convention working, not a lost decision.
Nothing else in the tree references 0745.

## §2 on the merged tree, run alone on a quiet machine

The sequence was read out of the merged `doc/todo/02-every-round.md`; this batch did not change
that file, so it is the same sequence 813 ran. Load at the start of the run was **1.94 / 3.79 /
10.08** — the one-minute figure is what matters and the five- and fifteen-minute ones are 816's
fuzzing campaign draining away. `ps` found no `cargo`, `rustc`, `pdfref` or `python` belonging to
any sibling; nothing ran beside the sequence.

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | clean, exit 0 |
| `cargo nextest run --workspace` | **2790 passed**, 0 failed, 18 skipped, 37.5 s |
| `cargo test --workspace --doc` | clean |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | clean, exit 0 |
| `cargo build --profile gates -p pdf-sandbox --bins` | built |
| corpus | 974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **63 incomplete**, 0 slow |
| `pdfref-hayro` | built |
| oracle | 1945 pages: **983 agrees, 61 contradicted, 836 ambiguous**, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render |
| text extraction | **487 of 508** documents fully in bounds; 10971/11163 words (98.28%) |
| `selection_census` | passes, 0 panicked |
| `accessibility_census` | 102853 elements, ratchets hold, 0 defects, 876/876 untagged pages honest |
| dates | 1545 strings, 1514 conform to §7.9.4 (97.99%) |
| xmp | passes |
| jpeg2000 | passes, 3 not comparable |
| quorra corpus | 957 pages: **932 agree, 22 differ, 3 refused, 17 not comparable** |
| `fixed_documents` | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | passes; 875 rows, 0 unreviewed |

Every headline the briefing predicted came back on the number, and the test count is the arithmetic
it should be: 817's tree printed 2789 with its own three new tests in it, so `main` at `22e1feef`
held 2786; 815 adds one and 817 adds three; 2790.

**815 changed `article.rs`'s bead comparison from a set to a sequence and nothing moved.** The
corpus's incomplete list is 63 either side, the oracle's three verdict counts are identical, and
quorra's four are. That is the expected result rather than a disappointing one — a thread's bead
*order* decides what §12.4.3's article navigation does, and no gate in this sequence navigates an
article. It is recorded because a change to a `pdf-model` source file with no pixel movement is
exactly what trap 1 says not to assume, and here it was checked rather than assumed.

The `viewer-qt` `-Wmaybe-uninitialized` lines appeared on the cold clippy run and not on the warm
one, which is what §2 says they are: gcc's, about `cxx-qt`'s generated bridge, not lints.

## §5 — the eight artefacts

One `cargo build --release` for the seven binaries, a second for the library, `cargo metadata` asked
from the main tree's own working directory so the target directory is `main`'s and not a
neighbour's (trap 15). All eight installed into `target/`:

`pdf-viewer`, `pdf-sandbox-worker`, `pdf-view-worker`, `pdf-viewer-gtk`, `pdf-viewer-qt`,
`pdf-viewer-confined`, `pdf-retrieve`, `libviewer_ffi.so`.

`tools/state.sh binaries` shows all eight newer than `HEAD`. The ninth file it lists, `safedocs`, is
an older round's crawl tool and is not one of §5's eight.

## §4 — every sweep against a pre-merge baseline, and every delta accounted

The baseline is `doc/todo/01`'s second method rather than its first: a checkout of its own at
`22e1feef`, with `doc/md/`, `doc/*.pdf` and the submodules symlinked in and a build directory named
on the command line, and the sweep binaries **built inside it** so that `CARGO_MANIFEST_DIR` points
at the base tree. Nothing in the working tree was touched, so no restore could throw an edit away.
Twenty-two sweeps in all: eighteen `conformance` binaries and `spec-errata`'s four.

**Identical, both sides:** `blockers`, `parts`, `quoted`, `unpriced`, `spec-errata emit`.

**Line-number shifts and nothing else** — the ledger grew two lines, `doc/todo/01` nine, and
`structure.rs` moved by 168: `capabilities`, `overstated`, `spec-errata check`, `spec-errata moved`
(bar one citation count, below).

The rest, with what moved and why:

| sweep | delta | attributed to |
|---|---|---|
| `ledger` | `implemented` 446 → 447; `silent 1` line gone | 817's row |
| `callers` | `pdf-model` 334 → 336 `pub fn`; named by a dependent 196 → 198; **138 unnamed unchanged** | 817's `structure.rs` — both new functions are called |
| `entries` | rows explaining themselves by an arrival 292 → 293; table entries stated 841 → 844; **172 reported over 48 rows unchanged** | 817's row gaining code |
| `owed` | §12.4.3 Articles 29 → 30 terms; total 4009 → 4014; **183 named by no source unchanged, reading list still 110 rows** | 815's ledger note |
| `unread` | `list_continuation_census.rs` enters the `/ID` witness lists; **hit counts unchanged** | 817's new example |
| `inapplicable` | naming-file counts up by one to four per term, ordering shifts with them; **row set identical, 55 confirmed claims unchanged** | 817's example, 815's `article.rs` |
| `counts` | sentences 9026 → 9091, attributed counts 454 → 456, all to clauses with no rows below; **150 agreeing, 58 uncountable and 4 double-counts unchanged** | new prose |
| `tables` | sentences 7053 → 7106, attributed keys 2620 → 2639, **all 19 new ones agreeing**; absent 101, denials 6, keyless 61 unchanged | new prose |
| `quotations` | documents 6844 → 6899 quotations, verbatim 2847 → 2869; ledger notes 1990 → 1993, verbatim 1524 → 1526; **diverging 38 and 2, both unchanged** | 815's and 816's new prose — no new misquotation |
| `overtaken` | decision records 632 → 635; **45 overtaken unchanged** | the three new ADRs |
| `pointers` | 9212 → 9289 pointers, live 5226 → 5268; **absent 102 and undefined 13 both unchanged** | new documents |
| `retired silent Tz` | mentions 1050 → 1058, **corrections 75 unchanged** | 817's and 815's own prose |
| `spec-errata applied` | places read 61076 → 61395, named errata 965 → 1005, **dropped `#NNN` 183 → 203**; headline 771 changes over 363 issues unchanged | 815's pdf-issues numbers, which this collection does not carry — the sweep counts them so a clean run says what it was clean over |
| `spec-errata moved` | §14.7.5 source citations 176 → 177 | 817's `tree.rs` |

**One delta is an artefact of the method and not of the merge**, and it is worth naming so the next
merge round does not read it as a finding: `pointers` resolves three `tmp/hayro/hayro-jbig2/…`
pointers in the merged tree and none in the baseline. `tmp/hayro` is untracked, so a fresh checkout
of the base commit does not have it. The absent count is unchanged either way.

**No sweep gained a hit.** Every changed line is a count that grew with the tree or a position that
moved under it.

## Worktrees and build directories

`tools/worktree.sh close 814 815 816 817 818base` — five checkouts and five build directories
(18 + 23 + 20 + 29 GB and the baseline's 272 MB), taken away as one act apiece. `list` afterwards
shows `main` alone, and `.claude/worktrees/` is empty. **None of the four rounds left a base
checkout behind**; the briefing's `r816base` did not exist by the time this round looked.

Two orphans did, from far earlier: `pdfv-759-before` and `pdfv-759-after`, 904 MB each, with no
worktree and no branch. `tools/worktree.sh list` cannot see them — it matches `pdfv-r*` and these
are `pdfv-759-*` — which is exactly the shape `doc/environment.md` warns about, a build directory
outliving the checkout with nothing to report it. Removed. **The instrument's blind spot is the
finding, not the gigabytes**: a round that names its baseline directory `pdfv-<something>` rather
than `pdfv-r<something>` gets no orphan warning, ever.

## Owed, carried forward

- **CI's verdict is now readable rather than pending, and only the owner can read it.** The push
  described at the top of this file put all four merges on `origin/main`; before it, the last run
  there was a pre-existing failure that said nothing about this tree. The agent user has no
  GitHub credential — `gh` asks for one and `git ls-remote` is refused — so `gh run list --branch
  main` is the owner's command. This round's own §2 sequence is green on exactly these bytes,
  which is the strongest statement available from here.
- **`doc/rfc/` awaits the owner's review**, untouched again.
- **`doc/todo/56` awaits the owner's decision on the exclusion amendment.** 814 settled the *source*
  question — ISO 21757-1 will not be bought, Adobe's *JavaScript for Acrobat API Reference* is the
  working source, pdf-issues secondary — and the amendment to `CLAUDE.md`'s JavaScript exclusion is
  a separate decision that has not been made. The settled half must not be read as the whole.
- **`doc/errata-read.md`'s Owed section is wrong, and two rounds have now found it.** It lists "a
  quotation in a Markdown file under `doc/`" among "the remaining populations nothing reads at all"
  and says counting it "is a round's work". `conformance --bin quotations` has read exactly that
  population since the five-hundred-and-fortieth; this round's run reports **6899 quotations in 1113
  documents, 2869 verbatim, 38 diverging**. 814 and 815 each found it and each recorded it rather
  than fixing it. It is a two-sentence correction and it is still owed.
- **Issue #700's 75 lines across 27 files** stand on retired Annex O table numbers. 815 wrote the
  predicate down and did not build the instrument; no sweep can see it.
- **`display_list` and `confined_wire` are under-run at their documented lengths** (816, ADR 0747),
  and eight of the fifteen targets gained under a hundred features between `INITED` and `DONE`.
- **`x509`'s corpus is thin** — 22 real certificates.
- **QUORRA_FEEDBACK §40** is unanswered.
- **The shared stash still holds one dead entry**, `ada5411`, fully superseded by `b5c1f180`.
  `tools/round.sh` will keep warning until the owner runs `git stash drop`; this round did not touch
  it, per `doc/environment.md`.
