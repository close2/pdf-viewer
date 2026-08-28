# 816 — What a campaign at the documented lengths buys, and a seeder that counts its own vocabulary

Date: 2026-08-28. Branch `round-816`, from `main` at `22e1feef`. Parallel round, worktree `r816`,
beside 814, 815 and 817.
ADR: [0747](../adr/0747-a-campaign-at-the-documented-lengths.md).
Touched: `fuzz/seed_confined_wire.py`, `tools/fuzz.sh`, `doc/verify.md`,
`doc/traps/instruments-and-reports.md` (trap 24), and two new files — `doc/adr/0747` and this one.
No Rust changed and no ledger row is touched.

ADR 0742 left three things in writing. This round takes two of them — the seeder that stopped short
of the transport's vocabulary, and the sentence that no target had been fuzzed for its documented
length — and answers the third as the question it is: *should the tree carry a seed corpus?* No, and
the section below says on what grounds.

## The seeder was seven questions short, not four

`doc/verify.md` said four and ADR 0742 repeated it. Read against
`crates/viewer-confined/src/protocol.rs` the number is **seven**: `Offset` and `FieldSelection`
(ADR 0225), `Fields` (0235), `FreeTextAt` (0238), `Highlight` (0357), `Readback` (0422) and `View`
(0737). The four-hundred-and-forty-fifth session counted correctly; three more arrived after it
counted, and the count it wrote down could not say so.

So the count is not written down any more. `query_kind`'s constants are read out of `protocol.rs`
by name — the treatment `MAGIC` in the same file already had since the seven-hundred-and-thirty-sixth
session — `QUERIES` is keyed by those names and states only the bytes *after* the discriminant, and
a name the module states which the script has no entry for stops the run before a worker is spawned.
The payload shapes stay this script's own reading of the format, which is the whole reason ADR 0223
wanted a second implementation.

`Command::Resize` and `Command::View` payloads are kept as seeds too, because `wire::command` is one
of the four decoders this target runs and the seeder kept none of what it *sent*; and the view is
set before the questions are asked, so `Query::View`'s answer is somewhere the reader was put.

### What the twenty-six seeds bought, measured

The re-seed wrote **26 new files** into a corpus of 8049, and nothing else had touched that
directory since ADR 0742 measured it (`find -newermt` over the interval: zero files). So this is a
clean attribution — libFuzzer's `INITED` line is the corpus's own coverage, before a single mutation:

| `confined_wire` at `INITED` | edges | features |
|---|---|---|
| ADR 0742's corpus | 6425 | 15 178 |
| the same corpus plus 26 seeds | **7520** | **17 101** |

**+1095 edges and +1923 features from 0.32% more files**, which is what a question nobody asks is
worth: the answers to seven of the transport's thirty-two questions had never been decoded by this
target at all.

### Trap 13, three plants, above commit `22e1feef`

The check is an instrument, so it was run against the defect. Each plant reverted, each message
distinct:

| plant | the script says |
|---|---|
| the `VIEW` entry deleted from `QUERIES` | `states 32 questions and this script asks 31. Unasked: VIEW.` |
| a `STRUCTURE` entry added that `query_kind` does not state | `asks for STRUCTURE, which … no longer states` |
| `mod query_kind` renamed in `protocol.rs` | *states no `mod query_kind` this script can read* |

## The campaign

Fifteen targets, each for the length `doc/verify.md` states for it, through `tools/fuzz.sh` so that
the invocation is that file's own, run **sequentially** — one target at a time, never `-jobs`.

**libFuzzer's `INITED` line is what makes this table say something.** It is the corpus's coverage
before any mutation, so the pair `INITED → DONE` is exactly *what the documented run length bought
on top of the seeds*, measured inside one run rather than across two.

| target | length | `INITED` | `DONE` | what the run added | seeds |
|---|---|---|---|---|---|
| `lexer` | 50 000 | cov 333, ft 2026 | cov 333, ft 2053 | 0 / +27 | 4269 → 4373 |
| `object` | 50 000 | cov 711, ft 4232 | cov 713, ft 4257 | +2 / +25 | 5401 → 5499 |
| `cmap` | 50 000 | cov 560, ft 2427 | cov 564, ft 2476 | +4 / +49 | 3422 → 3554 |
| `crypt` | 50 000 | cov 1252, ft 3967 | cov 1260, ft 4087 | +8 / +120 | 4142 → 4338 |
| `variable_text` | 50 000 | cov 7137, ft 21 774 | cov 7166, ft 22 057 | +29 / +283 | 6016 → 6312 |
| `forms_data` | 50 000 | cov 488, ft 1375 | cov 488, ft 1375 | **0 / 0** | 1302 → 1318 |
| `sfnt` | 50 000 | cov 500, ft 1641 | cov 502, ft 1723 | +2 / +82 | 953 → 1007 |
| `xmp` | 50 000 | cov 2244, ft 8922 | cov 2245, ft 8953 | +1 / +31 | 6277 → 6451 |
| `fragment` | 50 000 | cov 523, ft 2561 | cov 526, ft 2574 | +3 / +13 | 2111 → 2209 |
| `cms` | 50 000 | cov 487, ft 1211 | cov 493, ft 1221 | +6 / +10 | 732 → 771 |
| `document` | 50 000 | cov 5435, ft 24 173 | cov 5548, ft 24 846 | +113 / +673 | 11 529 → 12 171 |
| `display_list` | 600 s | cov 1794, ft 7292 | **cov 2678, ft 10 841** | **+884 / +3549** | 861 → 1512 |
| `x509` | 1 000 000 | cov 883, ft 1446 | cov 955, ft 1571 | +72 / +125 | 464 → 533 |
| `confined_wire` | 1 000 000 | cov 7520, ft 17 101 | **cov 8180, ft 20 290** | **+660 / +3189** | 8075 → 8458 |
| `page` | 50 152, `-fork=6` | cov 41 479, ft 222 148 | cov 41 992, ft 227 534 | +513 / +5386 | 9169 → 10 265 |

`page` has no `INITED` because a fork-mode parent does not print one; its left-hand column is the
parent's **first** reported counter, after the merge and before any child had found anything, which
is the same quantity. Its merge took 26 of its 51 minutes — the parent's first report
carries `time: 1577s` and its last `time: 3002s` — which is `doc/verify.md`'s own warning about
that target seen once more.

The campaign ran from a machine at a one-minute load average above 300 — three sibling rounds
building at once — down to 5, and no figure in the table is a timing. Wall clock per target and the
load each ran under are beside the logs.

### `tools/fuzz.sh` now prints the pair, and trap 24 gained the sentence

The `INITED` column above was read out of the logs by hand, which is the shape of a fact that ought
to be a command. So the wrapper prints it: the corpus's own coverage, and the subtraction. A
fork-mode parent has no `INITED` line, and the wrapper says that in words rather than subtracting
against a missing left-hand side — which would read as a run that started from nothing, the one
thing this wrapper exists not to say by accident (trap 11).

**Trap 13, two plants, above commit `22e1feef`** — the report is an instrument, so both arms were
run against the thing they are about, and the two calibration runs are the reason `forms_data` and
`cms` sit at slightly different corpus counts than the campaign table records:

| run | the wrapper says |
|---|---|
| `tools/fuzz.sh forms_data` (an ordinary run) | `the corpus alone gave cov: 488 ft: 1375, so this run added 0 edge(s) and 0 feature(s)` |
| `tools/fuzz.sh cms -- -fork=2 -runs=2000` | `no INITED line, so what the corpus gave and what the run added cannot be separated here; that is a fork-mode parent, which prints none` |

The first is also the `forms_data` finding reproduced on a second run and a second corpus state,
which is what makes it a statement about the target rather than about one seed.

### What the table says

**Crashers: none.** No `crash-`, `oom-` or `leak-` artefact was written by any of the fifteen —
`fuzz/artifacts` gained **nine files and all nine are `slow-unit-`**, one in `document` and eight
in `page`, and `page`'s fork parent carried `oom/timeout/crash: 0/0/0` on every one of its 109 jobs.

`doc/verify.md` says to read a slow unit in a release binary before believing it, and the one it is
cheapest to check says why. `document`'s, 198 KB, which libFuzzer called **25 s**, is **0.226 s** in
`target/pdf-retrieve` — main's binary, at the commit this branch is from, and this branch changes no
Rust. Twenty-five seconds is ASan, the debug assertions and six forks sharing a loaded machine,
which is the same order of ratio that block already documents for `page` (15 s → 0.8 s).

So: **a clean campaign, honestly reported.** Principle 3's "every crasher found becomes a permanent
regression test" had nothing to bind on, and the tree gained no fixture because there was nothing to
fix.

**The documented lengths are two very different lengths.** Ten of the fifteen ran in under two
minutes and eight of those added fewer than a hundred features. `-runs=50000` on `lexer`, `xmp`,
`fragment` and `forms_data` is over in seconds and moves the coverage by a rounding error — on
`forms_data` it moved nothing at all, twice measured. That is not a failure: those targets are
*saturated* against their corpora, and a run that finds nothing new against a mature corpus is the
result. But it means the number 50 000 is doing no work for them, and a round that wants to *find*
something in `pdf_syntax` has to bring new seeds rather than more iterations.

**Two targets are plainly under-run at the length written down for them**, and both are the ones
whose input is a *process* rather than a document:

- **`display_list`** gained **+884 edges and +3549 features in its ten minutes** — half again the
  coverage its whole seeded corpus had. A target still climbing that steeply at the end of its
  budget has not been given its budget's worth.
- **`confined_wire`** gained **+660 edges and +3189 features**, and it finished its million runs in
  **156 seconds**. The length was chosen when the transport was smaller; at 14 000 executions a
  second a million runs is two and a half minutes, and the target is still finding features when it
  stops.

Both are recorded here rather than acted on: changing a documented run length is a decision, and it
belongs to a round that can also say what the new one buys. The evidence for that round is the two
rows above.

**And `page` is the one target where the documented length is right and was worth its hour.** It
started from cov 41 479 / ft 222 148 and finished at cov 41 992 / ft 227 534 — +513 edges and +5386
features — and it wrote 1096 new units into its corpus, more than any other target. ADR 0742's
finding stands unchanged beside it: from *nothing* this target reaches cov 103 / ft 182, the same
two figures `document` reaches, because a fuzzer will not invent a header, a page tree and a
resource dictionary that agree with each other. Both facts are the same fact — `page` is worth an
hour **because** its corpus is 10 265 real documents, and it would be worth nothing without them.

### Which target's seeds are too thin, and what it would take

None of the fifteen is unseeded, which is the question ADR 0742 had to ask and this round did not.
The thin one is thin in a different sense: **`x509` has 533 seeds, the fewest of any target, and its
whole corpus is 1.9 MB.** Its million runs added 72 edges on 883, the third-largest proportional gain in
the table after the two process-input targets, which says it is *not* saturated, and `doc/verify.md`'s own block says why the seeds are
few: they are the 22 certificates the corpus's signatures carry, plus four generated by `openssl` to
reach the curve arms ADR 0689 added. A corpus of 22 real certificates cannot state what RFC 5280
allows, and the target's fastest path to more coverage is more certificates rather than more runs —
which is the same sentence as `page`'s, on a population nobody has collected. What it would take is
a source of certificates that is not this corpus's signers: the tree already has `fuzz/seed_x509.py`
to extract them, so the work is finding the input, not writing a script.

## What was done with the corpus, and whether the tree wants one committed

The campaign grew `fuzz/corpus` by **4074 units across all fifteen targets** — from `page`'s 1096
down to `forms_data`'s 16 — and the directory went from 1.2 GB to 1.4 GB. Nothing was committed, and
nothing should be:

- `fuzz/corpus` is **1.4 GB** on this disk, two thirds of it `page` (653 MB before this round) and
  `document` (302 MB). `.gitignore` excludes it and `tools/worktree.sh` links it, which is the
  arrangement ADR 0742 built and which this round used unchanged — so the 4074 units this campaign
  found landed in the **primary checkout's** corpus, where the next round will see them, which is
  the behaviour principle 3 asks for and the opposite of what a per-worktree copy would give.
- **What is committed is the recipe**, and that is the right unit: `fuzz/seed_page.py`,
  `fuzz/seed_nested_content.py`, `fuzz/seed_x509.py`, `fuzz/seed_confined_wire.py`,
  `viewer-confined`'s `list_over_the_wire` example, and `doc/verify.md`'s block per target. A recipe
  regenerates a corpus on any machine; a checked-in corpus is a snapshot that ages and that nothing
  can check.
- **A crasher is the exception, and it does not arrive as a corpus file either.** The precedent is
  ADR 0399's pair in `crates/pdf-model/tests/hostile_budgets.rs`: the artefact libFuzzer wrote was a
  mutation of a `SafeDocs` document that `doc/third-party-data.md` keeps out of this history, so what
  went in was a **generated minimal document** reproducing the defect under a test named for what it
  guards. That is the shape a future crasher takes here, and this round had none to take it.

So the answer to the question is *no*, on two grounds rather than one: size, and the fact that a
seed corpus committed once is a corpus nobody re-derives.

## Gates

Not a fifth round (`tools/round.sh` says so), and the change is documents and a Python script — no
Rust, no crate the gates rasterise with. So §2's core, both `fuzz/` lines included, plus the line
the change→gate map adds for a documents-only change:

| line | result |
|---|---|
| `cargo fmt --all --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | silent, beside the documented `proc-macro-error2` future-compat note and the cold-build `cxx-qt` gcc `-Wmaybe-uninitialized` lines |
| `cargo nextest run --workspace` | 2786 run, 2786 passed, 18 skipped |
| `cargo test --workspace --doc` | passed |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | silent |
| `cargo test -p conformance -- --nocapture` | passed, including `every_workspace_in_the_tree_is_formatted_compiled_and_linted_by_the_sequence` |

`--bin quotations` and `--bin pointers` are in the sweeps below, which is where a documents-only
change owes them.

**The machine was not quiet and this says so.** The campaign began at a one-minute load average
above 300 on 24 cores — three sibling rounds building at once — and ended at 5. Nothing in the
table above is a timing, and the fuzz figures are coverage counters rather than rates, so the load
costs wall clock and no verdict. It is recorded because §2's own warning is that a gate spawning
another program is a measurement of two programs, and none of the lines above spawns one.

## The §4 sweeps, before and after

Twenty-one sweeps over a pristine `git worktree` of `22e1feef` with its own build directory
(`r816base`, the second method in `doc/todo/01`), and again over this branch. Every exit status is
identical, `quoted`, `retired` and `unpriced` exiting 1 on both sides as they did in ADR 0742's run
— the first and third take an argument this run did not give them. **Five outputs differ and none
is a finding:**

- **`errata-applied`** reads 61 112 places against 61 076 — this round's prose. `0` name an erratum
  this collection carries on both sides, and the `#NNN` token count is identical at 1534.
- **`ledger`** differs only in the absolute path it prints. 875 rows, 0 new, both sides.
- **`overtaken`** reads 633 decision records against 632 — ADR 0747. Same 137 page-list notes over
  340 documents, same 45 overtaken, same three sub-counts.
- **`pointers`** reads 9229 path pointers against 9212, the seventeen being paths this round's two
  new files and its two edited ones cite. **`102 absent` on both sides**, which is the
  finding-shaped number, and `162 symbol pointer(s), 13 undefined` is identical.
- **`quotations`** reads 6849 in 1108 documents against 6844 in 1106 — the two new files. `2847
  verbatim` and `38 diverging` are identical on both sides, and neither new file appears as a hit
  in any sweep's list.

The remaining sixteen are byte-identical.
