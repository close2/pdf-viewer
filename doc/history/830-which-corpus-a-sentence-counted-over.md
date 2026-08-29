# 830 — Which corpus a sentence counted over, made into an instrument

Date: 2026-08-29. Branch `round-830`, from `main` at `0b6709f7`. Parallel round, worktree `r830`,
beside 829, 831 and 832.
ADR: [0758](../adr/0758-the-corpus-a-sentence-counted-over.md).
Touched: two new files under `tools/conformance/src/` — `undenominated.rs` and its binary — plus
`tools/conformance/src/lib.rs`, `crates/viewer-core/src/notes.rs` (one comment),
`doc/conformance/ledger.toml` (two notes, no status), `doc/todo/01-ledger-partial-rows.md`,
`doc/todo/02-every-round.md`, and two new files: `doc/adr/0758` and this one.

**A fifth round** — `tools/round.sh` says so from the branch name — so §2 ran whole and §5 rebuilt
and installed the eight artefacts.

`doc/rfc/` and `doc/todo/56` were not touched: both await the owner.

## What the round was asked and what it built

Four rounds had found the same defect, each by accident: a claim counted over `doc/pdf.js` in a
tree that now holds sixty-six times as many documents. ADR 0758 has the class, the predicate, the
departure from the twenty-second sweep's adjacency rule and what the sweep cannot see. What belongs
here is the running.

`cargo run --release -p conformance --bin undenominated` — **ten seconds**, where every other
sweep in `doc/todo/01` is a fraction of one. The ten seconds are the walk: its right-hand side is
the disk, and it counts the PDFs under `doc/pdf.js/test/pdfs`, under each directory of
`doc/corpora/` and under each of `corpus-cache/`. That walk's total is a check on itself — it
prints the same **67 460** that `find -L doc/pdf.js/test/pdfs doc/corpora corpus-cache -name
'*.pdf' | wc -l` prints and that ADR 0754's harvest ran over, arrived at by a different program.

## Trap 13 — calibrated above a commit, against four live defects

Not against plants. The pre-825 wordings of `doc/verify.md`, `crates/pdf-model/src/cms.rs` and
`doc/conformance/ledger.toml` were copied in from `3c259925` (`install -D` aside first, `cp` back
after — no `git checkout -- doc`, which takes the worktree's submodule symlinks away), the sweep
run, and the tree restored to a clean `git status`. All four print, and the corrected wordings
beside them do not.

**Two of the four changed the program, and both changes were made after the calibration rather
than before it:**

- **`doc/verify.md`'s "the eleven `/Contents` blobs the nine *signed* corpus documents hold"** sits
  inside a fenced `sh` block, as a `#` comment beside the command. Both Markdown readers in
  `tools/conformance` skip a fence, correctly — a shell line is not a sentence — so that claim was
  in **no population at all**. `fenced_prose` reads the comment lines of a fence and nothing else.
- **The same sentence defeated adjacency.** ADR 0709's `parts` may not read across a modifier;
  here a modifier narrows *what* is counted and leaves the population alone. Reading across at most
  three words, never past a function word or a unit, is what makes that sentence a finding. The
  rule is written in `MAX_MODIFIERS`' own doc comment with the sentence that earned it.

A third calibration is about the direction that **hides** a case, since denomination removes a hit:
`PDFBOX-4352-0.pdf` is a document of `doc/pdf.js` and spells the name of a corpus under
`doc/corpora/`, and a substring test read every sentence naming that file as a sentence naming that
corpus. Names are matched as tokens now, and the report tallies which name answered each
denominated claim so that a name answering by accident shows up as a number that grew.

## What the first run found

The summary line is the finding, and it is larger than any sentence in it: of the claims that
quantify over a corpus in this tree, **the large majority name no population**, and of those that
do, `974` and `pdf.js` answer nearly all. *The corpus* is an unwritten convention here, which is
why four rounds in a row were bitten by it and why no instrument could see it.

That is a backlog rather than a round. The command prints it, ranked; this round paid the top of
the list and the debt 828 named.

## What this round paid

| where | was | is |
|---|---|---|
| `doc/todo/01`, two record sentences | §12.8.5's absence "holds"; §12.8.3.3.2's "there are three" | both name `doc/pdf.js`, and the paragraph carrying the first now says what the pair proves — **a command without a stated population re-derives the same narrow answer** |
| §12.8.3.3.2's ledger row | "there are three" | three in `doc/pdf.js`, and the census re-run over all 67 460 finds **338 signature values in 325 documents**, 335 of the values in the crawl |
| `viewer-core`'s `notes.rs` | the same sentence beside the code | corrected with it, ADR 0754's own lesson applied |
| §12.8.2.2's ledger row, two sentences | "the corpus's one certification signature" | `doc/pdf.js`'s one — the row's later sentences already named the crawl's |

**No fixture is retired and no status moves.** 825's rule: a witness found in a crawl ranks a
format and cannot define one.

Each of those four is off the sweep afterwards, which was checked by re-running it: the ledger's
absence rung falls by three, the code rung by one and the document rung by four.

### The re-census, and the load it ran under

`cargo run --release -p pdf-model --example signature_algorithm_census -- @paths` over the 67 460,
**42 seconds** of wall clock at a one-minute load average of 43 on 24 cores with three sibling
rounds live. No conclusion rests on a rate; every figure it produces is a count. It reproduces ADR
0754's numbers exactly — 681 documents carrying a signature dictionary, 811 dictionaries, 796
readable, 20 document timestamps, 186 indefinite lengths, 145 certification signatures at
122/18/5 — which is what a confirmation looks like when the instrument has not changed and the
population has not either.

## The gates — §2 whole, a fifth round

Load average when the first reference-spawning line started: **27**, with three sibling rounds
live. That is not the quiet machine §2 asks for and it cannot be made one from here; it is
recorded because a loaded machine is a silent third in any gate that spawns a reference. **The
oracle's verdict counts are identical to the merge round's on `main`**, which is the check that
matters — a budget lost to load shows as *not comparable* growing, and it did not.

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | exit 0, no lint. The `warning:` lines are `viewer-qt@0.1.0:` gcc `-Wmaybe-uninitialized` on a cold build and `proc-macro-error2`'s future-incompatibility note — the two §2 names as not clippy's |
| `cargo nextest run --workspace` | 2810 run, 2810 passed, 18 skipped |
| `cargo test --workspace --doc` | 1 passed |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | exit 0 |
| `pdf-sandbox --bins`, `hayro-compare --bin pdfref-hayro` (gates profile) | built (trap 10) |
| corpus | 974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **63 incomplete**, 0 slow |
| oracle | 1945 pages: **983 agrees, 61 contradicted, 836 ambiguous**, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render |
| `text_extraction` (three gates) | **489 of 503** documents fully in bounds; 10951/11131 words (98.38%) |
| `selection_census` | 1000/1011 words selected (98.91%) over 453 documents |
| `accessibility_census` | 988 documents, 104 with structure, 90 tagged; 1502 of 1558 pages answer; no ratchet fell |
| `dates` | 1514 of 1545 date strings conform to §7.9.4 (97.99%) |
| `xmp` | 319 documents carry §14.3.2's stream, 318 read, 1 refused |
| `jpeg2000` | 14 codestreams byte-identical to OpenJPEG's decode |
| quorra corpus | Radeon 890M (RADV STRIX1), cpu coverage lane: 957 pages, **932 agree, 22 differ, 3 refused, 17 not comparable** |
| `fixed_documents` | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | **222 passed, 0 failed** — 209 before, plus this round's 13 |
| ledger | **875 rows, 0 new** — 447 implemented, 223 partial, 17 reported, 67 inapplicable, 8 writer-side, 113 out-of-scope. 0 `unreviewed`, 0 `silent` |

**The ledger binary was run as a formatter and produced nothing of its own.** Its rewrite leaves
`git diff` showing exactly the two notes this round edited.

**The oracle's reference cache was cold** — 9 hits of 6707 renders — because a worktree's build
directory is its own and 828 swept the root. That is wall clock and nothing else; the verdicts are
the verdicts.

## §4 — the sweeps, against a baseline of their own

A pristine `git worktree` of `0b6709f7` at `r830b`, its own build directory (`pdfv-r830b`), the
gitlink guard on at 6/6, closed with `tools/worktree.sh close 830b` afterwards. Twenty sweeps run
in both trees and diffed; **every exit status is identical** (`retired` exits 1 on both sides, as
it has since ADR 0742's run, taking nouns this run did not give it) and **no verdict worsened**:

| sweep | delta | why |
|---|---|---|
| `blockers`, `callers`, `capabilities`, `entries`, `inapplicable`, `overstated`, `owed`, `retired`, `unread`, `errata-moved` | **byte-identical** | |
| `parts` | **verdict byte-identical** — 583 cardinals, 39 agreeing, 48 on the closest rung, both sides; line numbers moved | this round's prose counts no part of this tree |
| `counts` | 9166 → 9182 sentences, 460 → 461 attributed counts, **151 → 152 the family agrees with**, 58 suspects and 4 double-counts unchanged | one new count, and the family agrees with it |
| `tables` | **every figure identical** — 2671 citations, 2503 agreeing, 101 absent, 6 denials, 0 under no such table | line numbers only |
| `quotations` | 6985 → 6996 in 1132 → 1133 documents; **2890 verbatim and 38 diverging both unchanged** | nothing written here claims to be a quotation and is not |
| `pointers` | 9423 → 9465 pointers, 5353 → 5373 live, 565 → 587 not carried; **102 absent and 13 undefined symbols unchanged** | the paths this round's prose cites |
| `overtaken` | 643 → 644 decision records; **45 overtaken and its 10/34/1 breakdown unchanged** | ADR 0758 |
| `ledger` | the absolute path only. 875 rows, 0 new, both sides | |
| `errata check/renumbered/applied` | 61 861 → 62 044 places read, **1048 naming an erratum and 203 unmatched `#NNN` unchanged on both sides** | this round's prose |

The structural caveat 825 recorded holds here too: several of these sweeps read `doc/todo/01`, so a
sweep whose population includes this round's own edits is reported with a one-step lag.

## §5 — the artefacts

All eight built with `--release` and installed, with `cargo metadata` asked for the target
directory rather than the path being written down (trap 15): it answered
`/home/AI/cargo-target/pdfv-r830/release`, which is **this worktree's** and not a neighbour's.
`pdf-viewer`, `pdf-sandbox-worker`, `pdf-view-worker`, `pdf-viewer-gtk`, `pdf-viewer-qt`,
`pdf-viewer-confined`, `pdf-retrieve` and `libviewer_ffi.so` are under this worktree's `target/`.

**They are this branch's, and the main checkout's `target/` is untouched deliberately.** §5's own
amendment is about exactly this: installing a worktree's build over the main tree's would put an
unmerged branch's program where a person runs `main`'s. Refreshing `main`'s eight is the merge
round's.

## Owed, named precisely, with the command that prints it

**The backlog this sweep found is the item.** `cargo run --release -p conformance --bin
undenominated` prints it ranked, and its five rungs are the order to take it in. Nothing about the
size of it is written here, for `CLAUDE.md`'s reason — the command counts it, and it will keep
counting it as the tree moves.

Three things about that backlog a later round should know before starting:

- **Most of it is not a defect.** A sentence written in a round that had no corpus but
  `doc/pdf.js` meant `doc/pdf.js`. What the sweep offers is which sentences a widening could have
  invalidated, and a denominator to check them against.
- **The cheapest payment is a clause family at a time**, because the census that answers one row
  usually answers its siblings — which is how this round paid four sentences off one 42-second run.
- **A round that widens a population owes the sweep**, which is the rule now written into
  `doc/todo/02` §4.

And carried forward from 828, untouched here: the push is the owner's; `QUORRA_FEEDBACK` §40 is
owed upstream; `Table A.19`'s foreign-standard designation is unchecked; the vertical-centre bound
is 831's; the build root's foreign directories and the shared stash's one dead entry are the
owner's.
