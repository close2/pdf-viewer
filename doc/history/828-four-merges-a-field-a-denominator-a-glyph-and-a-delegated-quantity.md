# 828 — Four merges: the field a reference could not reach, a denominator, one glyph, and a quantity the clause hands over

Date: 2026-08-29. A merge round: `main` at `3c259925` (823's result), four parallel rounds
integrated in round order, the whole of §2 run on the merged tree, §5's eight artefacts rebuilt and
installed, §4's sweeps taken against a pre-merge baseline in a checkout of its own, every worktree
closed with its build directory, and §5a's sweep taken for the first time in five rounds. No
feature work, and this round decided nothing that needs an ADR.

**This round did not push.** CI is green on the batch the owner pushed earlier —
`tools/round.sh` reads *CI's last run on main passed — 33216085536 Merge branch 'round-817'* — and
sending this one is the owner's.

`doc/rfc/` and `doc/todo/56` were not touched: both await the owner.

## The merges

| | branch | tip | merge commit | what it carried |
|---|---|---|---|---|
| 824 | `round-824` | `9717d9d0` | `ca2266f6` | `pdf-model`'s `action.rs` and `view.rs`, the new `examples/reset_form_census`, one ledger row, `doc/errata-read.md`, `doc/todo/01`, ADR 0753 |
| 825 | `round-825` | `a5e414d4` | `cababfb6` | `pdf-model`'s `cms.rs`, the new `fuzz/seed_cms.py` and `fuzz/seed_der.py`, `fuzz/seed_x509.py`, `.gitignore`, `doc/verify.md`, two ledger rows, ADR 0754 |
| 826 | `round-826` | `1e1b90e5` | `90768b41` | `pdf-model`'s `tests/oracle.rs`, `raster-compare`'s `lib.rs`, `doc/traps/oracle-and-references.md` (trap 26), `doc/HANDOVER.md`, one ledger row, ADR 0755 |
| 827 | `round-827` | `9fd2701e` (2 commits) | `6494468d` | `pdf-model`'s `tests/text_extraction.rs`, one ledger row, ADR 0756 |

**Four clean merges, no conflict resolved by hand.** `ort` auto-merged the one file all four
branches touched, `doc/conformance/ledger.toml`, and the five hunks are disjoint by construction —
§9.5 (826), §12.7.4.3 (827), §12.7.6.3 with its `code` array (824), §12.8.3.4.2 and §12.8.5 (825).
No two rounds wrote inside one row.

**Gitlinks.** `git ls-files -s doc/pdf.js doc/arlington-pdf-model` read `160000` before every one of
the four commits, and `conformance::every_declared_submodule_is_still_tracked_as_one` passes on the
merged tree. Every stage was git's own merge index; no `git add -A`, no `-u`, no `stash`. Each
message went in with `git commit -F`.

## The contact the briefing asked about, resolved by argument

824 changed *behaviour* in `pdf-model` and 827 changed a *test's population* in `pdf-model`, and the
briefing was right to ask whether they meet. They do not, and the reason is stronger than "different
files":

- 824's whole surface is `ViewState::reset_form` and the new `view::widgets_under` beneath it —
  §12.7.6.3's reset-form action, reached only by performing one.
- 827's surface is `tests/text_extraction.rs`'s judged population. Text extraction interprets a page;
  it performs no action.

Checked rather than assumed: `reset_form`, `widgets_under`, `ResetForm` and `widgets_by_field_name`
appear nowhere in `tests/text_extraction.rs` or in `tests/oracle.rs`. And the instrument that would
have caught contact anyway is the ratchet itself — 489 of 503 is what 827 measured alone, and it is
what the merged tree measures.

## The one ratchet that moved, and it moved to exactly its figure

`text_extraction`'s word-box gate on the merged tree:

> verdict: 10951/11131 matched words in bounds (98.38%), horizontal edges within 0.5 pt and vertical
> centres within 0.5 of the word's height; **489 of 503 documents fully in bounds**

827's move, unchanged by three neighbours. Nothing else moved: the corpus's incomplete list, the
oracle's three verdict counts, quorra's four, `fixed_documents`' row count and the ledger's row count
are all what the briefing named.

## The seeder verification (`doc/verify.md` §816's rule)

825 extracted the shared X.690 walk out of `fuzz/seed_x509.py` into a new `fuzz/seed_der.py`, so the
question the merge owes is whether the older seeder still harvests what it did. Both versions were
run over **one identical list of 67 460 documents** — the merged tree's `seed_x509.py`, and
`3c259925`'s in the baseline checkout:

```
941 distinct certificate(s): 712 first seen inside a signature, 220 stated by a document,
                              9 out of a fixture
```

Identical summaries, and `diff -r` over the two output directories reports **no difference at all**:
941 files, byte for byte, across the extraction. The harvest survived the move.

`tools/fuzz.sh --list` and `doc/verify.md` still agree: every one of the 15 targets resolves an
invocation out of that file, and none is refused. `cms` reports 2668 seeds — the corpus is shared
between worktrees by symlink and is appended to by every run, so it is at or above the figure 825
left.

**`seed_cms.py` was run on the merged tree and stopped deliberately.** It had walked the same 67 460
documents for twenty-one minutes and written 1168 CMS objects with no error when the gate sequence
came due; §2's *run nothing beside it* is not a rule to trade against a re-derivation, so it was
killed **by pid** (`kill 3425`, never `pkill`) and not restarted. What the merge needed from it — that
the extraction into `seed_der.py` is behaviour-preserving — is what the `seed_x509.py` comparison
above proves, since that is the shared code.

## The gates

The whole of §2, in order, on the merged tree, on a machine whose load average was **0.98** when the
first reference-spawning line started and which ran nothing else of this round's. One foreign process
was present and is named because it would otherwise be a silent third: a `quorra_gpu` test binary
under `/home/AI/cargo-target/quorra/`, **fifteen days old and at 0.0% CPU**, belonging to another
tree. It was not killed — it is not this round's child.

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | exit 0, no lint. The 28 `warning:` lines are `viewer-qt@0.1.0:` gcc `-Wmaybe-uninitialized` inside `rust::cxxbridge1::Vec<T>::Vec()` on a cold build, plus `proc-macro-error2`'s future-incompatibility note — the two §2 names as not clippy's |
| `cargo nextest run --workspace` | 2797 passed, 0 failed, 18 skipped |
| `cargo test --workspace --doc` | 1 passed |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | exit 0, silent |
| `pdf-sandbox --bins`, `hayro-compare --bin pdfref-hayro` (gates profile) | built (trap 10) |
| corpus | 974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **63 incomplete**, 0 slow |
| oracle | 1945 pages: **983 agrees, 61 contradicted, 836 ambiguous**, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render; 100.0% reference-cache hit rate |
| `text_extraction` (three gates) | **489 of 503** documents fully in bounds; 10951/11131 words (98.38%) |
| `selection_census` | 1000/1011 words selected (98.91%) over 453 documents; 11 drags did not select the word under them |
| `accessibility_census` | 988 documents, 104 with structure, 90 tagged; 1502 of 1558 pages answer; no ratchet fell |
| `dates` | 1514 of 1545 date strings conform to §7.9.4 (97.99%) |
| `xmp` | 319 documents carry §14.3.2's stream, 318 read, 1 refused |
| `jpeg2000` | 14 codestreams byte-identical to OpenJPEG's decode |
| quorra corpus | Radeon 890M (RADV STRIX1), cpu coverage lane, glyph quantum 1/16: 957 pages, **932 agree, 22 differ, 3 refused, 17 not comparable** |
| `fixed_documents` | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | 209 tests, all pass — including `every_declared_submodule_is_still_tracked_as_one`, `every_gate_in_the_sequence_answers_for_the_sandboxed_decoder` and `every_workspace_in_the_tree_is_formatted_compiled_and_linted_by_the_sequence` |
| ledger | **875 rows, 0 new** — 447 implemented, 223 partial, 17 reported, 67 inapplicable, 8 writer-side, 113 out-of-scope. **0 `unreviewed`, 0 `silent`** |

**The ledger binary was run as a formatter and produced no diff.** `cargo run -p conformance --bin
ledger` on the merged tree rewrote the file byte-identically: the four rounds' hand edits already sat
in clause order with the generator's own shape, so there is no formatter output to commit.

## ADRs

0753, 0754, 0755 and 0756 are all present, one per round, and **0757 is unused** — the reserved
second-ADR floor was not spent. `tools/state.sh counts` reports 643 decision records, four more than
the baseline's 639.

## §5 — the artefacts

Rebuilt with `--release` from the main tree and installed into `target/`, with `cargo metadata` asked
from the main tree's own cwd rather than from a worktree's (trap 15's shape): the target directory it
answered is `/home/AI/cargo-target/pdf-viewer/release`, which is the main checkout's and not a
neighbour's. All eight are in place: `pdf-viewer`, `pdf-sandbox-worker`, `pdf-view-worker`,
`pdf-viewer-gtk`, `pdf-viewer-qt`, `pdf-viewer-confined`, `pdf-retrieve`, and `libviewer_ffi.so` from
its own `cargo build --release -p viewer-ffi`.

**`tools/round.sh` will report them older than `HEAD` after this, and that is correct and not a
debt.** They were linked from `6494468d`, the last commit of this batch that contains a line of Rust;
what is newer is this file, and §5's own amendment says a round that moved a document does not pay
for a whole-graph fat link at the end of it. The next round that measures anything relinks, which is
the rule §5 actually states.

## §4 — the sweeps, against a baseline of their own

A pristine checkout of `3c259925` at `.claude/worktrees/r828b`, with its own build directory
(`/home/AI/cargo-target/pdfv-r828b`), the same submodule and corpus links a round's worktree gets,
and the gitlink guard on. Seventeen sweeps run in both trees and diffed. **Every delta is accounted
for, and no sweep's verdict worsened:**

| sweep | delta | why |
|---|---|---|
| `blockers` | count unchanged; one line number moved | 826 added ~105 lines above it in `oracle.rs` |
| `capabilities` | 65 → 66 sentences, 46 → 47 witnessed | 827's new §12.7.4.3 ledger note. The sweep's "lacking" match points at `cff.rs`, which is its own noise shape |
| `counts` | 9132 → 9166 sentences, 459 → 460 attributed counts, **150 → 151 the family agrees with** | one new count, and the family agrees with it |
| `entries`, `inapplicable`, `overstated`, `owed`, `parts`, `unread` | file counts +1/+2; `parts` sees the handover's trap row gain `26` | the new `examples/reset_form_census.rs`, and 826's trap 26 |
| `pointers` | 9365 → 9423 pointers, **102 absent unchanged, 13 undefined symbols unchanged** | three lines read `tmp/hayro/...` in the main tree and "no file of that name" in the baseline checkout — a property of the checkout, not of the merge |
| `quotations` | 6935 → 6982 in documents, 2874 → 2890 verbatim; **38 diverging unchanged**, ledger notes' **2 diverging unchanged** | 47 new quotations, none of them a divergence |
| `tables` | 2651 → 2671 key citations, 2483 → 2503 agreeing; **101 absent, 6 denials and 0 under-no-such-table all unchanged** | all 20 new citations agree with the table they name |
| `quoted` | 237 → 239 figures read, **123 → 125 confirmed**, 101 contradicted unchanged | 826's two new figures are both ones the gate prints, at the gate's own precision — which is §4's rule for a note quoting a gate |
| `unpriced` | **byte-identical** | no note gained a page whose bound it does not name |
| `overtaken` | 45 overtaken, and the 10/34/1 breakdown, both unchanged; 639 → 643 decision records; 340 → 335 documents | `SELECTION_BELOW_FLOOR` came *off* the sweep because 827 rewrote it and cited its own ADR in it, which is exactly the rule §4 states; `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE` joined on the sweep's documented noise rung, ADR 0756 being newer than its 0744 and mentioning `issue6127.pdf` |
| `spec-errata check/moved/renumbered/applied` | verdicts all unchanged at 0; 61 644 → 61 861 places read, 1637 → 1654 `#NNN` tokens | the batch's new prose |

`retired` was run over the four nouns this batch's corrections are about — `object identity`,
`document timestamp`, `no witness`, `reference form` — 129 mentions over 4 nouns, 3 carrying both
shapes. The one hit worth reading is ADR 0754 quoting the wording it retired, which is this sweep's
oldest documented false positive. **`cms.rs` no longer states the retired sentence at all**, which is
the check that mattered.

## §5a — the build root, and the judgement this round owned

`tools/worktree.sh list` reported the root at **241.9 GB**, over twice §5a's threshold, and four
consecutive rounds had correctly declined to sweep it because siblings were live. This round had
none, so the question was finally answerable — and the answer separates two things the number does
not:

- **What this project's tools made**, which is this round's to sweep: `pdf-viewer` (82 GB), and the
  five per-round directories `pdfv-r824`, `-r825`, `-r826`, `-r827` and this round's own `-r828b`
  (86 GB between them), every one of which `tools/worktree.sh` created.
- **What they did not**, which stays untouched and is named below for the owner: `quorra` (43.7 GB),
  `quorra-main` (9.9), `quorra-mask-round` (9.9), `quorra-a21540` (1.5), `probes-round` (9.5),
  `hayro` (807 MB), `jpxprobe` (50 MB), `pdfref-survey` (604 KB) — about **75 GB** that no checkout in
  this repository names. ADR 0752's whole point is that this instrument can classify them and cannot
  judge them, and a sweep that took them would be a round deleting another project's work on the
  strength of a directory listing.

So: the five worktrees were closed with `tools/worktree.sh close`, which takes each checkout and its
build directory as one act, and §5a's own command was run over the main checkout's three profiles —
`rm -rf /home/AI/cargo-target/pdf-viewer/{debug,release,gates}`, **never `tmp/`**, which holds the
reference-render cache the oracle's 100% hit rate comes out of. The cross-target directories
(`x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, 9.9 GB between them)
were left alone as well: §5a names three profiles and those are `doc/verify.md`'s, and a sweep should
be the command the section states rather than one a round invents.

Afterwards `tools/worktree.sh list` names **one worktree, `main`**, no `pdfv-r*` directory at all, and
a root of **88.4 GB** — under §5a's threshold for the first time in five rounds, of which the 75 GB
that is not this project's is now the large majority. The main checkout's own directory is 13.1 GB:
`tmp/`, the three cross-target directories, `miri`, `criterion` and `iai`.

The cost is one cold build for the next round, which §5a prices at about three minutes. §5's eight
artefacts are unaffected — they are `install`ed *copies* under the repository's own `target/`, not
links into the build root, and both were checked present after the sweep.

**Round 826 reported a worktree `r8240` it had not created; there is none.** It was 824's own
baseline checkout, closed by that round. `git worktree list` and the build root agree: nothing is
left behind by any round of this batch.

## Owed, and what is the owner's

- **The push.** `main` is four merges ahead of `origin/main`; sending it is the owner's.
- **`doc/rfc/` and `doc/todo/56`** both await the owner's decision and were not touched.
- **`QUORRA_FEEDBACK` §40** is still owed upstream.
- **`Table A.19`'s foreign-standard designation** is still unchecked.
- **The vertical-centre bound's defect**, of the same family as the word-box one 827 fixed and
  deliberately left untaken — recorded in ADR 0756, and the next round on that instrument's ground
  inherits it.
- **The shared stash's one dead entry.** `tools/round.sh` still reports the stack non-empty;
  `doc/environment.md` records that `stash@{0}` is fully superseded by `b5c1f180` and that dropping
  it needs the owner's permission.
- **The build root's foreign directories**, listed above: about 75 GB under
  `/home/AI/cargo-target/` that belongs to `quorra`, `probes-round`, `hayro`, `jpxprobe` and
  `pdfref-survey`. Only the owner can say whether any of them is finished with.
- **A fifteen-day-old idle `quorra_gpu` test binary** (pid 707311, 0.0% CPU) is still running from the
  quorra build root. It disturbed nothing measured here, and it is not this tree's process to kill.
- **`doc/todo/01`'s two record sentences about §12.8.5 have not taken 825's widened denominator.**
  Both say "no corpus document carries a document timestamp … and that one holds", written by the
  six-hundred-and-forty-first session over the 974. §12.8.5's ledger row now names its denominator
  explicitly and names the crawl's twenty timestamps beside it, so nothing in the ledger is wrong —
  but those two sentences are exactly the shape ADR 0751 and ADR 0754 are about, one file over. A
  merge round does no feature work, so it is written down rather than fixed.
