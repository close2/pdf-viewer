# 823 — Four merges: the brace that delimited everywhere, the designation nothing read, seeds rather than seconds, and the populations our instruments assume

Date: 2026-08-29. A merge round: `main` at `3ec61db7` (818's result), four parallel rounds
integrated in round order, the whole of §2 run on the merged tree, §5's eight artefacts rebuilt and
installed, §4's sweeps taken against a pre-merge baseline in a checkout of its own, and every
worktree closed with its build directory. No feature work, and this round decided nothing that
needs an ADR.

**This round did not push.** The owner pushed `main` during round 818 and CI went green on that
batch during 822 — `tools/round.sh` reads *CI's last run on main passed — 33216085536 Merge branch
'round-817'* — so origin carries the previous batch with a good verdict and this one is the owner's
to send.

`doc/rfc/` and `doc/todo/56` were not touched: both await the owner.

## The merges

| | branch | tip | merge commit | what it carried |
|---|---|---|---|---|
| 819 | `round-819` | `192f692d` (2 commits) | `c2db4fb6` | `lexer.rs`, `object.rs`, `parser.rs`, `function.rs`, a new `pdf-syntax` test file, four ledger rows, `doc/errata-read.md`, `doc/todo/01`, ADR 0749 |
| 820 | `round-820` | `5d5df80f` | `06b3602f` | `citation.rs`, `clause.rs`, `spec-errata`'s new `renumbered.rs` and its `main.rs`/`lib.rs`, three Annex O ledger rows, `doc/errata-read.md`, `doc/todo/01`, `doc/todo/02`, ADR 0750 |
| 821 | `round-821` | `9041864e` | `d343f617` | `fuzz/seed_x509.py`, `doc/verify.md`, ADR 0751 — no Rust |
| 822 | `round-822` | `45a6bb73` (2 commits) | `44bbbd27` | `tools/round.sh`, `tools/worktree.sh`, `tools/state.sh`, `doc/environment.md`, `doc/todo/02` (§5a), `doc/traps/instruments-and-reports.md` (trap 25), `doc/HANDOVER.md`, ADR 0752 |

**Four clean merges, no conflict resolved by hand.** `ort` auto-merged the three files two branches
touched — `doc/conformance/ledger.toml` (819's §7.2.3, §7.3.5, §7.10.5.2 and §12.3.3 against 820's
§O, §O.2.1 and §O.2.2), `doc/todo/01-ledger-partial-rows.md`, and `doc/todo/02-every-round.md`
(820's errata-subcommand line against 822's §5a paragraph) — with no hunk in common.

**Gitlinks.** `git ls-files -s doc/pdf.js doc/arlington-pdf-model` read `160000` before and after
every merge, and `conformance::every_declared_submodule_is_still_tracked_as_one` passes on the
merged tree. Every stage was by explicit path; no `git add -A`, no `-u`, no `stash`.

## `doc/errata-read.md`, the one contact the briefing predicted, resolved by argument

Both 819 and 820 wrote to this file and `ort` merged them without asking. A merge that produces no
conflict marker has still not shown that the two sentences agree, so both halves were read back out
of the merged file:

- **819's half is two edits.** The Owed section's bullet claiming "[t]he remaining populations
  nothing reads at all" is struck through and corrected — `--bin quotations` has read the Markdown
  population since the five-hundred-and-fortieth session, and a table *cell*'s quotation is read by
  construction because `conformance::prose::Conversion` folds `doc/md/`'s table rows into the same
  normalised body as its prose; what is genuinely unread is narrowed to a quotation that **spans**
  two cells, since the `|` between them survives the fold. Beside it, a new closing section: the
  ranking's seventeenth use, its table of two clauses, the brace finding, and the three notes on
  the rule.
- **820's half is two minimal places.** One new paragraph at the end of the sixth-blindness section
  — `spec-errata renumbered` is built, it pairs through §12.5.6.2's `/IRT` group rather than through
  the page, the written-down shape was nine parts noise, and the second grounding *ranks* rather
  than filters. And a one-clause tense fix in the *fifteenth* use's table, where §O.2.2's row said
  "no instrument in this tree can see it" and now says it could not "until `renumbered` was built".

**They do not contend, and each is load-bearing for the other's neighbour.** 819's correction is
about which *quotation* populations are read; 820's paragraph is about which *designation*
population is, and its tense fix is a sentence 820's own work made false. Both stand, all four
markers are present in the merged file, the retired sentence is gone, and the file reads in order
— 819's Owed correction near the top, 820's paragraph in the middle, 819's new section at the end.

## The widened citation population found nothing, and the briefing's caution was one step out

The batch briefing predicted that the conformance gate "now checks designations it never checked
before". **It does not, and the tree is right rather than the briefing** — this is worth writing
down because a merge round told to look for a finding can manufacture one.

`Scan::designations` is a *second* population beside `Scan::tables`, not a widening of it.
`read_tables` still pushes the digits that open a designation into `tables`, which is what the
conformance gate checks, and 820's own test
`a_designation_is_every_caption_shape_and_the_numbered_population_is_unchanged` pins both halves.
A grep for every consumer of `designations`, `designated_table_title` and `captions_table` finds
them in `tools/spec-errata/src/renumbered.rs` and nowhere else. So the gate saw exactly what it saw
before, and passed.

**The refactor underneath it did move, and was checked empirically.** `ClauseIndex::table_title`
used to match a `Table {n} -` prefix by hand and now goes through the shared `caption_of`, which
parses the designation and requires it to be the whole of what precedes the dash. Over the whole
tree that resolution is unchanged: `--bin tables` reports **absent 101, denials 6, keyless 61, 0
under no such table** on both sides of the merge, and its agreeing count grows by exactly the twelve
citations the batch's new prose added (2471 → 2483). A caption the old matcher resolved and the new
one did not would have landed in `absent`, and none did.

## §2 on the merged tree, run alone on a quiet machine

The sequence was read out of the merged `doc/todo/02-every-round.md`. Load at the start was
**1.35 / 1.26 / 2.17**, and `ps` found no `cargo`, `rustc`, `pdfref`, `nextest` or `python`
belonging to any sibling. Two long-lived processes were on the machine and neither competes for the
processor: the owner's `vorta` backup daemon, and a `quorra_gpu` test binary of somebody's from
fifteen days ago, sleeping at 0.0% CPU with 176 threads. It is not this round's child and was left
alone.

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | clean, exit 0 |
| `cargo nextest run --workspace` | **2795 passed**, 0 failed, 18 skipped, 35.4 s |
| `cargo test --workspace --doc` | clean |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | clean, exit 0 |
| `cargo build --profile gates -p pdf-sandbox --bins` | built |
| corpus | 974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **63 incomplete**, 0 slow |
| `pdfref-hayro` | built |
| oracle | 1945 pages: **983 agrees, 61 contradicted, 836 ambiguous**, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render |
| text extraction | **487 of 508** documents fully in bounds; 10971/11163 words (98.28%); all four tests pass |
| `selection_census` | passes, 0 panicked |
| `accessibility_census` | 102853 elements, ratchets hold, 0 defects, 876/876 untagged pages honest |
| dates | 1545 strings, 1514 conform to §7.9.4 (97.99%) |
| xmp | passes; 319 documents carry §14.3.2's stream, 318 read |
| jpeg2000 | passes, 14 byte-identical, 3 not comparable |
| quorra corpus | 957 pages: **932 agree, 22 differ, 3 refused, 17 not comparable** |
| `fixed_documents` | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | passes; 875 rows, 0 unreviewed, no `silent` line |

The status breakdown sums to the row count exactly: 447 implemented + 223 partial + 17 reported +
67 inapplicable + 8 writer-side + 113 out-of-scope = **875**, with **0 unreviewed in every clause
from 7 to 14**.

The test count is the arithmetic it should be: `main` held 2790, 819 adds two lexer and writer
tests and 820 adds three, and the merged tree runs 2795.

**819's lexer change moved nothing a gate can see, and that is the result rather than a
disappointment.** A name containing braces now lexes as one name, so the corpus, the oracle, the
text lines and quorra were the four to watch. Every one of their figures is identical to 818's, to
the last count: 63 incomplete, 983/61/836 with the same four sub-counts, 487 of 508 and the same
10971/11163, and 932/22/3/17. The reason is that `/A{B}` is a name almost no document writes — the
change is a correctness fix to a token boundary that decides a dictionary key, not a change to what
is drawn — and the branch's own calibration proves the fix is load-bearing from the other side:
with the braces put back the lexer test fails with `[Name([65]), Keyword([66]), Integer(1)]` where
the clause asks for `[Name([65, 123, 66, 125]), Integer(1)]`. Recorded because trap 1's rule is that
a change to a first-row crate with no pixel movement is checked, never assumed.

The `viewer-qt` `-Wmaybe-uninitialized` lines appeared on the cold clippy run, exactly as §2 says
they do: gcc's, about `cxx-qt`'s generated bridge, not lints. The lint run's exit status under
`RUSTFLAGS="-D warnings"` was 0.

## `doc/verify.md` and `tools/fuzz.sh` still agree

816's rule is that `tools/fuzz.sh` takes its invocation out of `doc/verify.md` so the two cannot
drift, and 821 changed both a length and the x509 seeding recipe. On the merged tree
`tools/fuzz.sh --list` prints an invocation for every one of the fifteen targets and refuses none:
`confined_wire` at `-runs=4000000`, `display_list` at `-max_total_time=600`, `x509` at
`-runs=1000000` over **1530 seeds** — 821's harvest, present in the shared corpus. No target is
listed as having no line in `doc/verify.md`.

## ADR verification

| number | branch | present |
|---|---|---|
| 0749 | 819 | yes — *Two delimiters that only delimit inside a type 4 program* |
| 0750 | 820 | yes — *The blindness a table number hides in* |
| 0751 | 821 | yes — *A recipe that named one corpus* |
| 0752 | 822 | yes — *The populations our own instruments assume* |
| 0753 | — | **unused, as reserved** |

`ls doc/adr/` ends at 0752 and nothing in the tree references 0753. Each of the four rounds took one
number and each says in its own second line which of the block was a sibling's, which is the
parallel-rounds numbering convention working.

## §5 — the eight artefacts, and the staleness flag cleared

One `cargo build --release` for the seven binaries and a second for the library, with
`cargo metadata` asked from the main tree's own working directory so the target directory is
`main`'s and not a neighbour's (trap 15). All eight installed into `target/`: `pdf-viewer`,
`pdf-sandbox-worker`, `pdf-view-worker`, `pdf-viewer-gtk`, `pdf-viewer-qt`, `pdf-viewer-confined`,
`pdf-retrieve`, `libviewer_ffi.so`.

`tools/round.sh` had been reporting `target/pdf-viewer` older than `HEAD` for several rounds and now
prints **✓ target's binaries are at least as new as HEAD**. The ninth file `tools/state.sh binaries`
lists, `safedocs`, is an older round's crawl tool and is not one of §5's eight.

**One thing about that check a merge round should know**: it compares a binary's mtime against
`HEAD`'s *commit time*, so committing this file after §5 makes it fail again — with binaries that
are byte-current for every line of code on `main`, the only later commit being a document. The
honest repair is to run §5's invocation once more rather than to touch anything: both builds come
back in a fifth of a second, having nothing to do, and the install refreshes the mtimes. That is
what happened here, and it is a structural artefact of the order §5 and §6 are written in rather
than a defect in either.

## §4 — every sweep against a pre-merge baseline, and every delta accounted

The baseline is a checkout of its own at `3ec61db7`, with `doc/md/`, `doc/*.pdf` and the submodules
symlinked in and a build directory named on the command line, the sweep binaries **built inside it**
so that `CARGO_MANIFEST_DIR` points at the base tree. Nothing in the working tree was touched.
Twenty-three sweeps: eighteen `conformance` binaries and `spec-errata`'s five, `renumbered` being
new this batch.

**Byte-identical, both sides:** `blockers`, `callers`, `overstated`, `unread`, `quoted`,
`unpriced`, `spec-errata emit`, and — bar line numbers — `spec-errata check`, whose ninety-two
changed lines are every one of them a position in `doc/errata-read.md`, `doc/todo/01`,
`parser.rs` or `spec-errata/src/lib.rs`, with not a word of any hit changed.

**No sweep gained a hit.** Every changed line is a count that grew with the tree, a position that
moved under it, or the new instrument's own report. The rest, with what moved and why:

| sweep | delta | attributed to |
|---|---|---|
| `ledger` | the path only — the base tree's is under `.claude/worktrees/r823base` | the method |
| `entries` | rows explaining themselves by an arrival 293 → 294; **844 table entries, 172 reported over 48 rows, all unchanged** | 819's or 820's ledger notes |
| `overtaken` | decision records 635 → 639; **45 overtaken unchanged** | the four new ADRs |
| `capabilities` | capability sentences 196 → 198, witnessed 161 → 162, about the program 118 → 120 | 820's `renumbered.rs` module prose. One of the two is unwitnessed and is a sentence saying what the tool *does not* do — "[i]t renames nothing" — which is a claim of no capability and cannot have a witness. Read and left. |
| `counts` | sentences 9091 → 9132, attributed counts 456 → 459, the three new ones all under clauses with no rows below; **150 agreeing, 58 uncountable, 4 double-counts unchanged** | new prose |
| `tables` | sentences 7106 → 7150, attributed keys 2639 → 2651, **all 12 new ones agreeing**; absent 101, denials 6, keyless 61 unchanged | new prose — and the check on 820's `caption_of` refactor, above |
| `quotations` | documents 6904 → 6929 quotations over 1114 → 1122 files, verbatim 2869 → 2874; ledger notes 1993 → 1994, verbatim 1526 → 1527; **diverging 38 and 2, both unchanged** | the batch's new prose — no new misquotation |
| `pointers` | 9289 → 9365 pointers, live 5268 → 5322; **absent 102 and undefined 13 both unchanged** | new documents |
| `owed`, `inapplicable`, `parts`, `retired`, `spec-errata moved` | naming-file counts up by one, orderings and line numbers shifting with them; **every hit set identical** | new prose and the moved source lines |
| `spec-errata applied` | places read 61395 → 61644, named errata 1005 → 1033; **dropped `#NNN` 203 unchanged**, headline unchanged | 819's and 820's errata prose |
| `spec-errata renumbered` | **new** — the baseline prints its usage line and exits 1 | 820's instrument, below |

**`parts` prints trap 25 as reached**, which is the sweep reading `doc/HANDOVER.md`'s trap index
after 822 added a trap to it. It is not a hit; the row's list simply ends `24, 25` now.

**The new instrument's first run on the merged tree** reports *11 of 2865 annotations strike a
designation the conversion captions and pair it with another; 2 strike it inside the clause that
captions that very table, which is the rung to read* — Issue #700's two, `Table Annex O.3` with 33
source citations and `Table Annex O.4` beside it, with the nine noise annotations (Issue #124's four
array indices, Issue #133's two NOTE numbers, and the rest) ranked below rather than filtered out.
Exactly what 820 said it would print, on a tree that now has three other rounds' work in it.

**One artefact of the method, named so the next merge round does not read it as a finding.** The
scratchpad is shared between rounds and is not per-session (`doc/environment.md`), and the baseline
sweep directory this round chose already held a file from **28 August at 21:59** — `exits.txt`,
belonging to a sibling round that had run its own baseline sweeps into the same path. Every file
this round compared was written by this round, checked by timestamp before the comparison was
believed, and the stray file appears in the listing as *missing in main* and nothing else. It is the
documented hazard springing harmlessly, and the mitigation the document already states — name a
scratch file after the round — would have avoided even the moment of doubt.

## Worktrees, build directories, and what the new classification says

`tools/worktree.sh close 819 820 821 822 823base` — five checkouts and five build directories
(21.6 + 22.6 + 19.3 + 17.6 GB and the baseline's 271 MB), taken away as one act apiece.
`.claude/worktrees/` is empty afterwards and `list` shows `main` alone. All five had the gitlink
guard on, 6 of 6, for their whole lives.

**822's rewritten `list` earns its place on its first merge round, and what it reports is not
orphans.** There are none: every `pdfv-r*` directory was live and all five are gone. What the
widened walk shows instead is the shape §5a's threshold is actually about —

| | before the close | after |
|---|---|---|
| the main checkout's `pdf-viewer` | 81.0 G | 81.0 G |
| this batch's five worktrees | 81.3 G | — |
| **directories no checkout here names** | 75.3 G | 75.3 G |
| **the whole root** | **237.6 G** | **156.3 G** |

The 75.3 GB is `quorra` at 43.7 G, `quorra-main` and `quorra-mask-round` at 9.9 G each,
`probes-round` at 9.5 G, `quorra-a21540` at 1.5 G, `hayro` at 807 M, `jpxprobe` at 50 M and
`pdfref-survey` at 604 K. The old glob could not name one of them; `tools/state.sh disk` now prints
the root's 157 G beside the checkout's 81 G, which is the second number §5a asks for. Closing this
batch took the root from 238 GB to 156 GB and it is **still past the hundred-gigabyte threshold**,
with roughly half of what remains belonging to directories this project's tools did not make and
cannot judge. That is a decision about other work, not a sweep this round may make silently — see
the owed list.

`tools/round.sh` passes every check on the merged tree, and reports 7 superseded build scripts
naming gone checkouts, which its own comment calls noise: those are other rounds' and cargo will not
reach for them.

## What the batch says together

Four rounds, four instruments turned on their own assumptions.

- **819 — the brace that was a delimiter everywhere.** §7.2.3's table lists ten delimiter
  characters and the clause's own introducing sentence scopes two of them to type 4 PostScript
  calculator functions; the lexer held all ten for the whole of its life, so `/A{B}` was a two-byte
  name and three tokens where the standard makes it one name of four, and by §7.3.5 those are
  different objects. The one place the braces do delimit tokenises itself.
- **820 — the designation nothing read.** Looking for the ground under a renumbered table found
  that `read_tables` parsed digits after `Table ` into a `u16` and stopped, so `Table Annex O.3`,
  `Table D.2` and `Table 125a` were no reference to anything. And the renumbering itself was
  *declined*, on the tree's own standing argument: `doc/md/` is the published text citations
  resolve against.
- **821 — seeds rather than seconds.** A fuzz target's corpus is worth more than its wall clock:
  fixing the x509 seeding recipe's argument list bought +1342 edges from seeds alone, where a whole
  million-run campaign on the same target had bought +72.
- **822 — the populations our own instruments assume.** A hand-written population can name a thing
  that never existed, and finding nothing there prints as a pass; a glob can answer "did I leave one
  behind" while reading as "what is on the disk". Trap 25.

The common sentence is one this batch can state four ways: **an instrument's population is a claim
about the world, and a narrow one and a clean tree produce the same output.** 819's was the
standard's delimiter set, 820's the shape of a table's name, 821's the corpus a fuzzer starts from,
822's the directories a script walks. None of the four was found by a gate, and three of them were
found by asking an instrument what it was looking at.

## Owed, carried forward

- **`doc/rfc/` awaits the owner's review**, untouched again.
- **`doc/todo/56` awaits the owner's decision on the exclusion amendment.** 814 settled the *source*
  question; the amendment to `CLAUDE.md`'s JavaScript exclusion has not been made.
- **QUORRA_FEEDBACK §40** is unanswered.
- **`Table A.19` is an unchecked designation pointing at a foreign standard.** It is the one table
  this tree cites that ISO 32000-2 does not caption, attributed in both places to ISO/IEC 15444-1,
  and nothing checks a non-numeric designation for correctness. A gate would need the
  foreign-standard rule `read_citations` already has for a SECTION SIGN. Named by 820 and still a
  round of its own.
- **`doc/verify.md`'s `cms` block has the same single-submodule defect 821 fixed for `x509`** —
  named here and not taken, because it is a seeding recipe rather than a merge's business.
- **The build root is past §5a's threshold and half of it is not this project's.** 156 GB after this
  batch closed, of which 75 GB belongs to `quorra*`, `probes-round`, `hayro`, `jpxprobe` and
  `pdfref-survey` — directories no checkout in this repository names, one of which has a fifteen-day-old
  test binary still holding it open. The 81 GB that *is* the main checkout's can be swept by §5a's
  own command at the cost of one cold build; the other half is a decision about other work and
  should be asked before it is taken.
- **The shared stash still holds one dead entry**, `ada5411`, fully superseded by `b5c1f180`.
  `tools/round.sh` will keep warning until the owner runs `git stash drop`; this round did not touch
  it, per `doc/environment.md`.
- **CI has 818's batch and not this one.** This round's §2 sequence is green on exactly these bytes,
  which is the strongest statement available from here; the push is the owner's.
