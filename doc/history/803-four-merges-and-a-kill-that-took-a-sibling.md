# 803 — Four clean merges, and the kill that took a sibling

Merge round, on `main` from `46289075`. Merged `round-799` (`fe1f7a54`), `round-800`
(`2ba7c626`), `round-801` (`ef8effe1`), `round-802` (`b9d91fd2`), in round order, each with
`--no-ff` and each a single commit on its branch. **All four clean — no conflict in any file**,
including `doc/conformance/ledger.toml`, which 799 (§7.7.2 and §7.7.3.2), 800 (§7.3.3 and
§7.10.5.2) and 802 (§8.7.3) touched on disjoint rows; the ort strategy reconciled them and the conformance gate on
the merged file passed, so this batch has no reconciliation commit. The expected code contact did
not materialise either: 799's and 802's `pdf-model` edits are in `page.rs` and
`content/pattern.rs`, 800's are in `pdf-syntax`'s lexer and `pdf-model`'s `function.rs`, and 801
touched nothing any of the three did.

Then the full §2 sequence on merged `main`, one documentation correction this round owns, §5's
install, the §4 sweeps against a pre-merge baseline, and the four worktrees closed.

## The correction this round owns: `pkill -x` is not the safe form

`doc/environment.md`'s process-table bullet has said, for as long as it has existed, that the fix
for `pkill -f` matching a sibling's path is **`pkill -x <exact-name>`**. It is not one, and this
batch is the proof: stopping its own gate run, round 800 ran `pkill -x cargo` and took round
**799**'s mid-gate build with it, which was then watched rebuilding its `gates` profile from near
scratch. `doc/history/800-what-the-lexer-admits.md` records it under "One thing this round got
wrong, recorded because a sibling paid for it".

The argument is one sentence and the bullet had never made it: **`-x` narrows the *pattern*, not
the namespace.** It bounds the match to the executable's name and says nothing whatever about
whose process it is, so for a program every round runs under one name — `cargo`, `rustc`,
`cargo-nextest`, `pdfref` — it narrows nothing at all. What is safe is the handle a round already
holds: **kill your own children by PID, or by the process group of a script you started**
(`kill "$pid"`, `kill -- -$(ps -o pgid= -p "$pid" | tr -d ' ')`). `pkill -x` survives only for a
name no sibling round runs, which is not a list worth guessing at.

The rewritten bullet keeps the loud failure (`pkill -f` against a path), names the quiet one
(`pkill -x` against a shared program name), keeps the exit-144 tell, and adds the gentler shape
800 also hit twice: a `pgrep -f <script>` wait-loop matches **its own command line** and reports
a finished job still running. Committed separately from the merges (`18f618e7`).

**One thing the correction leaves standing, deliberately.** ADR 0734 offers
`pkill -9 -x pdf-view-worker` as the instrument for killing the confined worker, and
`pdf-view-worker` is exactly the kind of name a sibling round *could* be running. It is a record
of what 801 did rather than a rule, so it was not rewritten here; the next round to reach for it
now has the paragraph that says why to use a pid instead.

## Gitlink verification

`git ls-files -s doc/pdf.js doc/arlington-pdf-model` read `160000` for both paths immediately
before each of the five commits this round made, and reads `160000` now. Every stage named its
paths (`git add doc/environment.md` was the only `add` in the round; the merges staged
themselves). No `git add -A`, no `git add -u`, no `git stash`.
`cargo test -p conformance`'s `every_declared_submodule_is_still_tracked_as_one` passed on the
merged tree.

## ADR verification

`main` ended at 0731 pre-merge; the batch brought **0732** (799), **0733** (800), **0734** (801),
**0735** (802) — one per round as the briefings reserved, no collision, and nothing at 0736 or
above.

## Gates (full §2 sequence on merged `main`, quiet machine, nothing beside it)

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy, `-D warnings`, `--workspace --all-targets` | clean, exit 0 (the only `warning:` lines are gcc's on the cold `cxx-qt` bridge and the `proc-macro-error2` future-incompat note) |
| `cargo nextest run --workspace` | **2773 tests run, 2773 passed, 18 skipped** |
| `cargo test --workspace --doc` | ok, 24 suites |
| fuzz `check --bins`, `-D warnings` | clean — `confined_wire` still compiles against 801's reshaped boundary |
| `cargo build --profile gates -p pdf-sandbox --bins` | ok (trap 10) |
| pdf-model corpus | ok — 974 documents, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **66 incomplete**, 0 slow |
| `pdfref-hayro` build | ok |
| oracle | ok, exit 0 — 1945 pages, **983 agree, 61 contradicted, 836 ambiguous**, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render; 100.0% reference-cache hit rate, 0 renders produced |
| text_extraction (three gates) | ok — pdftotext 99.2% (22834/23013), PDFBox 99.8% (14257/14281) both orders, position verdict 10971/11163 (98.28%), **487 of 508** documents fully in bounds |
| selection_census | ok — readback differs on 0, caret misnames 0, drag 1000/1011 (98.91%, printed not ratcheted), panicked 0 |
| accessibility_census | ok — 102853 elements, 876 of 876 untagged pages answering the honest empty tree, 0 disagreeing lines, panicked 0; no ratchet moved |
| dates, xmp, jpeg2000 | ok |
| render-quorra corpus | ok — 957 pages, **932 agree, 22 differ, 3 refused, 17 not comparable** |
| fixed_documents | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | 207 passed — 11744 citations, 875 subclauses, 0 `unreviewed`, ledger prose naming 2894 clauses and 281 tables |

**The nextest count is the batch's exact arithmetic**: the base's 2755 plus 799's three, 800's
four, 801's ten and 802's one — eighteen `#[test]` items added across the four diffs, none
removed, and 2755 + 18 = 2773.

**Every verdict this batch could have moved landed where its round left it, and that is the
interaction check rather than a formality.** 799's page-tree guard, 800's lexer change and 802's
stroke construction all touch what the corpus and the oracle see. The corpus's incomplete list is
**66**, which is 802's move off 67 and nobody else's; its composition prints **the file 56, neither
one 9, this reader 1** — the reader's-fault class down to one mechanism, again 802's, and
`scorecard_reduced.pdf` is off the list. The oracle's three headline verdicts are byte-identical
to the pre-merge triple, which is what 799's and 800's changes were expected to do: an empty
`/Kids []` and a number too large for a double are both shapes the pdf.js corpus does not carry on
a rendered page. So no verdict moved that is not attributable, and none moved that nobody claimed.

## §5 — the eight artefacts

Rebuilt in `--release` from the **main tree's** build directory, asked for rather than written
down: `cargo metadata --no-deps | jq -r .target_directory` printed
`/home/AI/cargo-target/pdf-viewer`, run with the shell in `/home/cl/projects/pdf-viewer` (the
value depends on the *working directory*, not on `--manifest-path`, which is the trap-15 shape the
section warns about). One invocation for the seven binaries, a second for `viewer-ffi`'s
`cdylib`; `install -Dm755` for all eight. `tools/state.sh binaries` shows all eight at this
round's timestamp.

**`tools/round.sh` will nonetheless report `target/pdf-viewer is older than HEAD`, and it is right
without anything being wrong**: the two commits after the install are this round's own documents —
the `pkill` correction and this file — so the binaries are the merged code exactly. A rebuild
would relink identical output. The next round to take a *measurement* still owes §5, because the
next executable change will be its own.

## §4 — the sweeps, against a pre-merge baseline

Every sweep was run on `46289075` before the first merge and again on the merged tree, and the two
outputs diffed. **Four sweeps are byte-identical** — `entries`, `unread`, `callers` and
`overstated` — and errata `check` and `moved` are identical too. Every remaining delta is accounted:

| sweep | delta | attribution |
|---|---|---|
| `blockers` | one line number, `pdf-viewer-confined.rs:716 → :865` | 801's additions to `pdf-viewer-confined.rs`, above the line holding §7.6.4.1's blocker |
| `capabilities` | three line numbers; one witness path moved, `transparency.rs:4 → pattern.rs:1159` for the word *isolation* | 802's group-as-region construction is now the nearest source naming it |
| `pointers`, `tables`, `counts`, `quotations` | line numbers inside `doc/todo/01-ledger-partial-rows.md` and `doc/environment.md` only | 799's additions to the todo and this round's own bullet rewrite |
| `inapplicable` | naming-file counts up by one to four; §7.3.3 enters three cousin lists | 800's new `numeric_form_census` example and its §7.3.3 row |
| `owed` | `222` rows and `182` debts named by no source both **unchanged**; §8.7.3 moves up the list at 6 terms instead of 3, every one named | 802's rewritten note names three more terms, all of which the tree already carries |
| `quotations` | 6732 quotations in 1088 documents (was 6703 in 1080), 1979 in ledger notes (was 1974) — **diverging still 38 and 2** | the four ADRs and four history files this batch adds; no new misquotation |
| `parts` | line numbers only; no cardinal changed its verdict | no part was added to this tree |
| errata `applied` | 60371 places read, was 60082; **0 name an erratum this collection carries**, unchanged | the merged text is larger |
| `overtaken` | **48 → 49 overtaken**, and this is the one substantive delta — see below | 802's ADR 0735 |

`quoted` and `unpriced` take the oracle's log and so have no pre-merge left-hand side; run against
this round's log, `unpriced` is **clean** (93 failing bounds over 61 pages, 93 of them named by the
note that holds the page, 0 not) and `quoted` reports its standing level of 101 contradicted
figures over 36 notes. No branch in this batch touched `crates/pdf-model/tests/oracle.rs`, and the
oracle's verdicts did not move, so both sides of that pair are `main`'s as they were.

### The one delta that is a finding: three page-list notes ADR 0735 overtook

`overtaken` gained a hit and extended two more, all of them ADR 0735's:

- `AMBIGUOUS_TILING_CELL_CLIP` (`oracle.rs:5236`, newest ADR cited 0495) — 0735 is about
  `issue16038.pdf`, which that note's prose argues. **New to rung 2.**
- `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` (`oracle.rs:7927`, newest cited 0419) — 0735 joins at the
  head, about `issue12295.pdf`.
- `DIFFERS_IN_SHAPE` (`render-quorra/tests/corpus.rs:557`, newest cited 0535) — 0735 joins at the
  head, about `issue12295.pdf` and `issue16038.pdf`.

This is the nineteenth sweep working exactly as designed and the rule it comes with pointing at
802: *a round that rewrites a note cites its own ADR in it*. 802 did not rewrite those notes — it
had no reason to, since none of their verdicts moved — so the gap is bibliographic rather than a
wrong sentence, and reading three tiling notes against a new construction is a round's work and
not a merge's. **Filed as owed below**, with the observation that the honest fix is to read
whether 0735's group-as-region construction changes what any of those three notes diagnoses,
and to cite it either way.

## Worktrees closed

`tools/worktree.sh close 799 800 801 802` — all four checkouts and all four build directories gone
in one act, confirmed by `list`, which now names only `main` and no orphan directory.

**And that settles a briefed owed item in the other direction, so the tree wins over the
briefing.** The round was told the shared build directory stands at ~158 GB, past §5a's hundred.
It does not: **158 GB is the whole of `/home/AI/cargo-target`**, which holds four quorra
checkouts and two probe directories belonging to other work. This project's own build directory,
which is what §5a's threshold and its `rm -rf` name, is **81 GB** after the four closes — under
the threshold, so no sweep is owed. The two populations had been conflated. What *is* worth a
glance is `pdfv-759-before` and `pdfv-759-after` (0.9 GB each): orphan worktree build directories
that `worktree.sh list` does not name, because its pattern expects `pdfv-rNNN` and these are
`pdfv-NNN-*`.

## What the batch is about, taken together

Four rounds, and three of them are the same question asked at three different boundaries: **what
does a guard actually count?**

**799 — the array that states no children.** A page-tree node's `/Kids` was accepted on
`as_array().is_some()`, which is a claim about the *entry* and not about its population. A node
writing `/Kids [] /Count 3` therefore passed the guard, was believed about its count, and produced
no pages; the same shape in an intermediate node shifted every later page by the count it claimed.
The guard counts what it found now. The round's second half is the sharper one for this project's
method: it fixed the **errata recipe's own step 3**, whose attribution had been landing annex
annotations on a clause 14 row — an instrument mis-filing its findings is worse than a missing
finding, because the finding is there and points somewhere else.

**800 — what the lexer admits, and the number that was never zero.** A *conforming* real number
too large for a double came back as `Integer(0)`. That is the worst answer in the available set,
and the reason is worth keeping: zero is a legal value, so nothing downstream could tell a
magnitude that had been lost from one the file stated. It saturates to the double's extremes and
says so. The same round found a PostScript calculator function compiling `{ inf }` to a pushed
infinity, which reached a colour component as NaN — refused and reported now. Fifty million fuzz
runs clean over the lexer.

**801 — two refusals that outlived their reasons.** A refusal is *about* something, and
`Content::Refused` carried neither the feature list nor the target it was about, so a refusal a
zoom in had earned survived the zoom out that lifted it. The second is the harder one: a confined
worker that dies used to leave a window that simply stopped answering. `viewer_confined::Resuming`
turns that into a refusal of the page, bounded at three consecutive restarts — a death made
legible instead of silent, which is principle 3's shape at the process boundary.

**802 — the stroke a pattern colours.** §8.7.3's row had said since the fifty-third session that a
stroke coloured by a tiling pattern was owed, and gave three true reasons: the cell would be
replayed across the stroked outline, the outline is the backends' to compute, and computing it in
`pdf-model` would be a fourth stroke expander in the one crate that deliberately has none. Every
sentence was true and **the conclusion did not follow**, because the region a stroke covers is not
only a path: §11.5.2's group-as-region construction makes a group holding the stroke alone into
that shape, and it travels as a `Command::Stroke` each backend expands with the expander it
already has. No backend and no display-list type changed. That is the ledger's sixth refusal
shape — a row whose stated reason was never disproved and whose *inference* was.

The three of them and the `pkill` correction share a moral this round did not go looking for: **a
guard, a refusal, a lexer's answer and a process filter are all claims about a population, and
each of the four was narrowing the wrong thing.**

## Owed, named

- **CI's verdict awaits the owner's push.** `main` is far ahead of `origin/main` and unpushed by
  instruction; `origin/main`'s last run is a pre-existing failure
  (`gh run view 33121581297 --log-failed`). Nothing in this batch was measured against CI.
- **`doc/rfc/` awaits the owner's review** and was not touched.
- **QUORRA_FEEDBACK §40** is still open.
- **`doc/todo/15`'s next piece**: restoring magnification and scroll position across a worker
  restart. 801 made a death a refusal; what a *reader* loses when the worker comes back is a view
  question sitting exactly on the confined boundary, which is why it is the next piece and not
  part of 801.
- **The device path still has no interrupt to offer.** A long draw on the graphics device cannot
  be asked to stop.
- **The three tiling page-list notes ADR 0735 overtook** — `AMBIGUOUS_TILING_CELL_CLIP`,
  `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`, `DIFFERS_IN_SHAPE`. Read each against 802's construction
  and cite 0735 either way; the nineteenth sweep will go quiet when they do.
- **`ConfinedError` is `#[non_exhaustive]` and says nowhere why**, while `doc/ui-boundary.md` says
  in as many words that **nothing** on this boundary is, deliberately, because it forces a
  catch-all arm on every host — and prices what that cost four consumers once already. Either the
  attribute is wrong or the document owes an exception; neither is a merge round's call.
- **Two rustfmt diffs under `fuzz/fuzz_targets/`** — `display_list.rs:92` and `x509.rs:38`, both
  edition-2024 reflows — are still there and are still invisible to `cargo fmt --all`, which does
  not reach a crate outside the workspace. Verified this round with
  `rustfmt --check --edition 2024 fuzz/fuzz_targets/*.rs`.
- **The owner's `git stash` entry** is still on the stack and still dead; only somebody with the
  permission can `git stash drop` it. `doc/environment.md` says why it is safe to ignore and
  costly to pop.
- **Errata: Issue #327's railroad diagrams reach neither errata instrument.** A diagram is not a
  quotation and not a moved clause, so `check`, `emit`, `moved` and `applied` are all silent about
  it by construction. Left for `doc/errata-read.md`'s owner.
- **`pdfv-759-before` / `pdfv-759-after`**, 0.9 GB apiece, are orphan build directories
  `tools/worktree.sh list` cannot see (its pattern is `pdfv-rNNN`). Not urgent at 81 GB, but they
  are the shape the environment file warns about.
