# 833 — Four merges, four names that resolved against the wrong thing, and the block closed

Date: 2026-08-29. A merge round on `main`, from `0b6709f7`. Four branches — `round-829`,
`round-830`, `round-831`, `round-832` — merged in round order, **no conflicts in any of the four**.

ADR: none. A merge round decides nothing of its own; what it decided here is *editorial* and is
below under "The block".

Touched, beyond the four merge commits: `doc/history.md` (the block summary, which is a closing
round's one exception to the no-append rule) and this file.

`doc/rfc/` and `doc/todo/56` were not touched: both await the owner.

## The merges

| branch | commits | contact with a neighbour | resolution |
|---|---|---|---|
| `round-829` | two | none | clean |
| `round-830` | one | `doc/conformance/ledger.toml`, `doc/todo/01` | auto-merged, both sides present |
| `round-831` | one | `doc/conformance/ledger.toml` | auto-merged |
| `round-832` | one | `doc/conformance/ledger.toml`, `doc/todo/01` | auto-merged, both sides present |

829 and 830 both wrote in `doc/todo/01`, in different sections — 829's *the rule's nineteenth use*
and 830's twenty-third-sweep entry — and git took both. 832 then added to the same file a third
time, again without contact. The ledger took four rounds' note edits the same way.

`git ls-files -s doc/pdf.js doc/arlington-pdf-model` read `160000` before each of the four commits.
Every stage was by explicit path; nothing was stashed.

**The ledger was regenerated with its own binary after all four merges and produced no diff at
all** — 875 rows, 0 new — so there is nothing of the formatter's to commit. That is the result to
want: it says four rounds' hand-edited notes came through the merge already in the form the
generator writes.

**ADRs.** 0757, 0758, 0759 and 0760 are present, one per round. **0761 is unused** — the highest
number in `doc/adr/` is 0760 — so the reserved second-ADR floor was not spent.

## The gates — §2 whole, on a quiet machine

One-minute load average when the sequence started: **0.56**. Nothing else of this round's was
running beside it; the four sibling worktrees were still open but idle, and were closed afterwards.
Every line exited 0.

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | exit 0, no lint |
| `cargo nextest run --workspace` | **2817 run, 2817 passed, 18 skipped** |
| `cargo test --workspace --doc` | 1 passed |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | exit 0 |
| `pdf-sandbox --bins`, `hayro-compare --bin pdfref-hayro` (gates profile) | built (trap 10) |
| corpus | 974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **63 incomplete**, 0 slow |
| oracle | 1945 pages: **983 agrees, 61 contradicted, 836 ambiguous**, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render; reference cache 6707 of 6707, 0 produced |
| `text_extraction` (three gates) | **493 of 503 documents fully in bounds; 11094/11131 words, 99.67%**; corpus 99.3% (24014/24193) and PDFBox 99.8% (14257/14281) |
| `selection_census` | 1000/1011 words selected (98.91%) over 453 documents |
| `accessibility_census` | 988 documents, 104 with structure, 90 tagged; 1502 of 1558 pages answer; 876 of 876 untagged pages honest; no ratchet fell |
| `dates` | 1514 of 1545 date strings conform to §7.9.4 (97.99%) |
| `xmp` | 319 documents carry §14.3.2's stream, 318 read, 1 refused |
| `jpeg2000` | 14 codestreams byte-identical to OpenJPEG's decode |
| quorra corpus | Radeon 890M (RADV STRIX1), cpu coverage lane, glyph quantum 1/16: 957 pages, **932 agree, 22 differ, 3 refused, 17 not comparable** |
| `fixed_documents` | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | **227 passed, 0 failed** (218 in the main binary) |
| ledger | **875 rows** — 447 implemented, 223 partial, 17 reported, 67 inapplicable, 8 writer-side, 113 out-of-scope; **0 `unreviewed` in every clause, 0 `silent`** |

### The two things a merge round is here to check

**831's ratchets moved, to exactly the values it stated and to no others.** Word boxes **489 → 493
of 503**, the verdict **98.38% → 99.67%**, and `SELECTION_BELOW_FLOOR` **14 → 10** — the ten
documents the gate now names are `TrueType_without_cmap`, `issue11555`, `issue18099_reduced`,
`issue20232`, `issue2391-2`, `vertical`, `bug1771477`, `issue14497`, `issue1905` and `issue6127`.
`JUDGED_FLOOR` did not move, and the new `CROSS_AXIS_FLOOR` holds. Nothing else in the table moved
from the figures the four branches reported in their own worktrees.

**829 changed a filter's behaviour and no gate saw it.** The ASCII85 refusals are new, and the
corpus's incomplete list, the oracle's seven verdicts, quorra's four and `fixed_documents`' 41 are
all at their standing values — which is what 829 predicted and is the one thing a worktree round
could not establish for `main`.

## §5 — the eight artefacts, into the main tree's `target/`

Built with `--release` in one invocation and installed. The build directory was **asked for**
(`cargo metadata --no-deps`, run with the main checkout as the working directory) rather than
written down, which is trap 15's own subject: it answered `/home/AI/cargo-target/pdf-viewer/release`,
the main tree's. `pdf-viewer`, `pdf-sandbox-worker`, `pdf-view-worker`, `pdf-viewer-gtk`,
`pdf-viewer-qt`, `pdf-viewer-confined`, `pdf-retrieve` and `libviewer_ffi.so` are all under
`target/` and all newer than `HEAD`.

830 deliberately left this to the merge round — it had installed into *its own* worktree's `target/`
— and that was right: installing an unmerged branch's program where a person runs `main`'s is what
§5's amendment is about.

## §4 — the sweeps, against a baseline taken with the tool rather than by hand

Twenty sweeps run in the merged tree and in a pristine checkout of `0b6709f7`, diffed line by line.
**The baseline was opened with `tools/worktree.sh open 833b` and then detached onto the base
commit**, which is 832's methodology finding applied: the script links `doc/md`, `doc/*.pdf`,
`doc/pdf.js`, `doc/arlington-pdf-model` and the corpora into the checkout, so `pointers` measures a
tree with the same data on disk and the large live↔not-carried movement 832 saw from a bare
`git worktree add` does not appear at all. The gitlink guard came up 6/6 and the detach left
`git status` clean. Closed with `tools/worktree.sh close 833b` afterwards, checkout and build
directory together.

**Exit statuses are identical on both sides** — `retired` exits 1 in both, as it has since ADR
0742's run — and **no defect count worsened anywhere**. Two verdicts *improved*, and both are
attributable.

| sweep | delta | why |
|---|---|---|
| `callers`, `entries`, `overstated`, `retired`, `unread` | **byte-identical** | |
| `parts` | **verdict byte-identical** — 583 cardinals, 39 agreeing, 544 not, 48 on the closest rung, both sides | this batch's prose counts no part of this tree |
| `blockers` | one line number | 832's doc-comment edits in `signature.rs`; 42 sentences, 12 expired, 18 holding, both sides |
| `inapplicable` | §7.4.3 joins a cousin list, `MAX_DEPTH` named by 23 → 24 files | 829 took §7.4.3 to `implemented`; the 67/299/55/244/236 verdict is unchanged |
| `owed` | 4024 → 4027 stated terms | the four rounds' ledger notes; **183 named by no source over 113 rows, and the 110-row reading list, all unchanged** |
| `counts` | 9166 → 9248 sentences, 460 → 461 attributed counts, **151 → 152 the family agrees with** | one new count, and the family agrees with it; 58 suspects and 4 double-counts unchanged |
| `quotations` | 6985 → 7029 in 1132 → 1140 documents, 2890 → 2910 verbatim; ledger notes 1998 → 2005, 1531 → 1535 verbatim | **38 diverging and the ledger's 2 diverging both unchanged** — nothing written in this batch claims to be a quotation and is not |
| `tables` | 7183 → 7238 sentences, 2671 → 2675 key citations, 2503 → 2507 agreeing | **101 absent, 6 denials and 0 under no such table, all unchanged** |
| `pointers` | 9423 → 9533 pointers, 5353 → 5385 live, 565 → 596 not carried | **102 absent and 13 undefined symbols unchanged**; the movement is four ADRs and four history files' worth of new citations, not the baseline artefact |
| `overtaken` | 643 → 647 decision records, 335 → **333** documents | four ADRs; the two documents are `bug868745.pdf` and `issue6605.pdf`, which 831 took off `SELECTION_BELOW_FLOOR` and which no other page list names — `issue1350.pdf` and `issue4665.pdf` left the same list and stay, because `oracle.rs` names them. **45 overtaken and its 10/34/1 breakdown unchanged** |
| `capabilities` | 198 → **197** capability sentences, 120 → 119 about the program | a finding *paid*: 832 rewrote `spec-errata::renumbered`'s module comment, and the hit it used to carry is gone. 162 witnessed, unchanged |
| `errata check`, `moved` | line numbers only | |
| `errata applied` | 61 861 → 62 246 places read, 1048 → **1070** naming an erratum | 829's six issue readings and 832's writing; **203 unmatched `#NNN` unchanged on both sides** |
| `errata renumbered` | 33 → 35 source citations, and **six "elsewhere" place counts fell** — 33→26, 54→36, 30→29, 33→26, 30→29, 54→36 | **832's foreign-table rule, seen from the outside.** Those rows count the places in this tree standing on a table number an erratum renumbers, and the references that belong to ISO/TS 32002's Table 3 and Table 4 are no longer among them. This is the merged evidence for that round's finding, in a sweep that had no way to report it before |
| `ledger` | the absolute path only | 875 rows, 0 new, both sides |

### `undenominated`, first reading on a tree with all four branches in it

The twenty-third sweep did not exist at `0b6709f7`, so it has no baseline and this is its first run
over a tree holding 829's, 830's, 831's and 832's prose as well as its own. Ten seconds:

> 2491 claims quantify over a corpus. 586 name a population — 225 by `974`, 200 by `pdf.js`, 56 by
> `crawl`, 24 by `curated`, 23 by `safedocs`, 17 by `submodule`, 14 by `pdfbox`, 12 by
> `format-corpus`, 6 by `65944`, 4 by `submodules`, 2 by `67460`, 2 by "both populations", 1 by
> `pdf20examples` — and 780 sit in a dated record; **1000 name none**, of which 397 are an absence or
> a uniqueness (93 in the ledger, 196 beside the code, 108 in a document) and 603 are a count; 125
> state a denominator no population has. 4 invocations walk a corpus and 1 of them walks some and not
> the rest. This tree holds 67 460 documents.

Two cautions for whoever takes the backlog. The 780 in a dated record are `doc/history/` and
`doc/adr/` and are **not** work: a record says what was true when it was written. And the reading
list is the other 1000, ranked by the sweep's five rungs, of which the top rung — a fenced
invocation that walks some of this tree's corpora and not the rest — is one instance and is the
cheapest thing on it.

## Worktrees

`tools/worktree.sh close 829 830 831 832` took all four checkouts and all four build directories
away as one act; **`r830`, which its round left open on purpose, is closed with the rest.** The
baseline `833b` went the same way. `tools/worktree.sh list` now shows the main checkout and nothing
else, and classifies the build root as:

- **the main checkout's**: `pdf-viewer`, 34.6 G;
- **not this script's — no checkout here names it**: `quorra` 43.7 G, `quorra-main` 9.9 G,
  `quorra-mask-round` 9.9 G, `probes-round` 9.5 G, `quorra-a21540` 1.5 G, `hayro` 807 M, `jpxprobe`
  50 M, `pdfref-survey` 604 K;
- **no orphans.**

**The root is 109.9 G, which is past `doc/todo/02` §5a's threshold, and it is not swept here.**
75.3 G of it is the eight directories this project's tools did not make and cannot judge — they were
named for the owner by round 828 and are still owed to them — and the remaining 34.6 G is this
tree's own three profiles, all of which were rebuilt during this round. Sweeping them now would buy
nothing and cost the next round a cold build.

## The batch — four names that resolved against the wrong thing

The four rounds took four unrelated subjects and each found the same shape underneath: **a
designation that resolved, silently, in a namespace nobody had stated.**

- **829 — the exponent the text layer lost.** §7.4.3's first "shall never occur" condition is
  *greater than 2³² − 1*, and it prints as *232 - 1* in the ISO PDF's text layer and in `doc/md/`
  alike, so no quotation gate in this tree could ever have seen the real bound. The decoder
  saturated in `u32` and turned every five-character group above the missing bound into four `0xFF`
  bytes without a word — about 3% of admissible groups — and the clause's third condition, a final
  group of one character, had never been implemented at all, under a ledger row claiming all three
  were refused. A bound is a name for a quantity, and this one had lost a glyph.
- **830 — a command without a stated population.** `--bin undenominated`'s predicate is one
  sentence: *a sentence quantifies over a corpus and does not say which*. Its right-hand side is the
  disk, so "the corpus" is judged against the corpora this tree actually holds. It paid four claims,
  and the sharpest is §12.8.3.3.2's row, which said "there are three" and now says three in
  `doc/pdf.js` — because the census over all 67 460 finds **338 signature values in 325 documents**.
- **831 — what the cross-axis measure was actually asking.** The difference of two word-box centres
  is the difference of two *baselines* plus half the difference of two *bands*; §9.4.4 owns the
  first and ADR 0323 Finding 3 already excluded the second as each extractor's own convention, so
  the measure could not report either alone. Table 120 decides which pages have a band the *file*
  states, and on the rest the measure is set aside — the measure, not the word, so neither the
  judged set nor the pair count moves.
- **832 — the table that resolved in another standard.** Twenty-one references to ISO/TS 32002's
  Table 3 and Table 4 were checked against ISO 32000-2's Table 3 and Table 4 and passed, because the
  numbers exist in both. The rule is positional, so the writing was brought to the rule rather than
  the rule loosened — a document cannot be carried across a comment, since `eddsa.rs`'s own first
  line names ISO/TS 32002 and cites ISO 32000-2's Table 260 eight words later.

Read together they are the block's thesis in miniature, which is why the block closes here: an
exponent, a corpus, a centre and a table number are all *names*, and each of the four resolved
against something nobody had written down.

## The block

**This merge round closes the run the owner asked for, and it closes the block that opened at 675.**
The summary is appended to `doc/history.md` beside the other five, which is a closing round's one
exception to the no-append rule.

The argument for closing here rather than letting it run:

1. **The boundary is real rather than arithmetic.** A block is a run of rounds with something in
   common, and the unit that ends here is the commission — the owner's run of rounds — not a count.
   Merge round 674 closed the previous block on exactly that footing.
2. **It is already three times the longest block ever closed** — 158 rounds against 50, 30, 30, 30
   and 20 — and a summary is only worth writing while one round can still hold the run in view. This
   one was at the edge of that. A block left open now would not be closable at all, and an
   unwritten summary is the one loss `CLAUDE.md` calls unrecoverable.
3. **It has a thesis, and this batch is its clearest instance.** 624–673 found instruments that
   could not see their populations; 675–832 found that the *sentences* do not state theirs, built
   the two sweeps that ask (`parts`, `undenominated`), and then found the same failure with a
   standard as the namespace. There is a sentence to write, which is the only real test of whether a
   block is a block.

What that summary contains: the denominator arc and its two sweeps; the host arc, which gave the
confined boundary its first window and the tree its seventh consumer without a new crate; the twelve
traps the block added, including the two that are one defect at two altitudes; the ECMAScript
exclusion examined by argument twice and left where it was; the parallel-round namespaces; and the
numbers, from 674's own printed figures to this round's.

## Owed, and to whom

- **`doc/rfc/` and `doc/todo/56` are the owner's** — a transform suite and a script engine, each a
  decision a round has prepared and may not take. Untouched here and untouched by all four branches.
- **`QUORRA_FEEDBACK` §40** is still owed upstream.
- **The em box's `(1.0, 0.0)`** — the whole nominal line above the baseline, so a highlight over a
  descriptor-less font covers no descender — recorded in ADR 0759 and deliberately not taken. It
  belongs to a round that measures selection rectangles.
- **`issue6127.pdf` is still undiagnosed**, and is one of the ten documents `SELECTION_BELOW_FLOOR`
  now names.
- **The `§` half of 832's hole is open.** `ISO 21757-1:2020 §9` still reads as a citation of ISO
  32000-2 §9, because `another_document` requires a number of digits and hyphens and the colon in a
  year defeats it. Found in round 814, recorded in `doc/todo/56`'s own table, and not closed by 832,
  which fixed the permissiveness of the same function and the `Table` arm's missing rule. It is
  latent today — the only place in the tree that writes such a citation is that table cell — which
  is exactly the condition under which it will be forgotten.
- **The eight foreign build directories** under `/home/AI/cargo-target/` are 75.3 G and were named
  for the owner by round 828; they are why the root is over §5a's threshold.
- **The shared stash's one dead entry** still needs `git stash drop`, which this account cannot run.
- **The push is the owner's.** With this round's own commit `main` is 32 commits ahead of
  `origin/main`, and it was not pushed.
