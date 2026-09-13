# 1009 — A trailer may not name a catalog the file does not hold

Date: 2026-09-12, finishing 2026-09-13. Branch: `batch-1006-1011`, worktree
`/home/AI/pdf-viewer-rounds`, shared with five sibling rounds (1006–1008, 1010, 1011).
ADR: [1028](../adr/1028-a-trailer-may-not-name-a-catalog-the-file-does-not-hold.md).

## What was asked

ADR 1024 §5's find: the `serialize` fuzz target, which had never run because `tools/fuzz.sh --list`
printed `NO INVOCATION` above an exit status of 0, aborted on its **first seed pass** over a
184-byte recovered file — `a file whose trailer names /Root must reach it`. Diagnose it, fix it
where the clause puts it, make the crasher a permanent regression test, run the target properly,
and then price the third extraction if there was room.

## What the defect was

The input's own `/Root` reaches nothing: no `endobj` closes object 1, so the scan recovers no object
1 and `Document::catalog` refuses the *input*. The recovery is not at fault — the trailer's claim is
what the bytes state, and the reader reports the consequence, typed, when a caller asks. The writer
was: §7.5.5's Table 15 gives `/Root` a **Type** of `dictionary` as well as requiring it, and
`serialize` checked only that an assembly named one. So it wrote a trailer naming object 1 above an
object 1 written as `null`.

`SerializeError::RootNotADictionary`, refused before a byte reaches the sink. The check follows
§7.3.10's chain as far as `Document::resolve` does — `MAX_REFERENCE_DEPTH` is `pub(crate)` for that
reason — because a `/Root` whose object holds another reference is a file this reader opens, and a
writer that refused it would be enforcing a stricter rule than the one it protects.

## The two fuzz runs

Both are `tools/fuzz.sh serialize`, which takes its invocation from `doc/verify.md`. **A fuzz target
cannot run under `tools/bounded.sh`** — ADR 1024 §5's warning, heeded rather than rediscovered.

| | corpus in | wall clock | result |
|---|---|---|---|
| `doc/verify.md`'s own `-runs=50000` | 1 134 seeds (1 133 from `fuzz/seed_page.py`, plus the crasher) | 420 s total, 286 s of it fuzzing after the ASan build | exit 0, no artifact; `cov: 6228 ft: 29481`, corpus 1 134 → 2 937 |
| `-runs=1000000 -max_total_time=900` | 2 937 | 901 s of fuzzing, 92 298 runs | exit 0, no artifact; `cov: 6462 ft: 31618`, corpus 2 937 → 4 778 |

142 298 runs and about twenty minutes of fuzzing in total, over a corpus that went from 1 134 inputs
to 4 778. Neither run produced an artifact; the one file in `fuzz/artifacts/serialize/` is still ADR
1024 §5's, which is now a seed and a unit test rather than a crash.

The first run's seed pass — `#1136 INITED`, which is where it aborted before — read every unit. The
corpus alone gave `cov: 5220 ft: 20078` at that point, so the first run found 1 008 edges and 9 403
features the seeds did not, and the second found 235 and 2 149 more on top of what it left.

## What the round did not do, and what it learned by not doing it

The second extraction was priced and **not taken**. `pdf-colour` collides with session 1008's slice
(`pdf-model/src/shading.rs` is theirs). The `Permissions` split — ADR 1020 §2's "one type" — turns
out to be one type *and* one predicate: `field_locks` and `field_mdp` are defined over *signed*
fields, and this tree's predicate for signed is `signature::read(…).is_some()`, the whole CMS
parser. Deciding what signed means without the verification stack is a §12.8 reading with a
population behind it, and taking it as a third item of a round would have been taking it quietly.
ADR 1028 §5 has the scope, the measurement and the blocker.

## Files touched

`crates/pdf-syntax/src/serialize.rs`, `crates/pdf-syntax/src/document.rs`,
`crates/pdf-syntax/tests/serialize.rs`, `doc/conformance/ledger.toml` (§7.3.10 and §7.5.5 rows),
`fuzz/corpus/serialize/` (the crasher, gitignored),
`doc/adr/1028-a-trailer-may-not-name-a-catalog-the-file-does-not-hold.md`, this file.

## The gates, and the one line about them worth keeping

The sequence `doc/todo/02` §2 asks of a `pdf-syntax` change is the whole of it. What ran and passed
here is the core scoped to this round's crates (`cargo fmt -p pdf-syntax --check`, `RUSTFLAGS="-D
warnings" cargo clippy -p pdf-syntax --all-targets`, `cargo nextest run -p pdf-syntax`, its
doctests, and both `fuzz/` lines), then `on_disk`, `pdf-model`'s `corpus`, `dates`, `xmp`,
`jpeg2000`, `fixed_documents`, `actions`, `save_round_trip`, `oracle` and `text_extraction`,
`pdf-archive`'s `corpus`, `render-raster`'s, both `viewer-core` censuses, and `launch_path` — with
every `--bins` build that precedes one (trap 10, four times).

Two things it did not get, and both are the shared worktree rather than this change.

**`raster_golden` fails, naming five documents, all "reports only (a change in the diagnosis, trap
37)"** — `bug1050040.pdf`, `issue11915.pdf`, `issue12418_reduced.pdf`, `issue13316_reduced.pdf`,
`issue5801.pdf`. Nothing this round touched can reach a rasteriser: `grep` over `pdf-model`,
`pdf-font`, `render-cpu` and `pdf-render` finds not one reference to `pdf_syntax::serialize`, and
the other edit is a `pub(crate)` on a constant. A sibling had 218 lines of
`crates/pdf-font/src/program.rs` open at the time.

**`pdf-transform`'s seven writer walks are the gates that *can* see this change, and they ran —
on the second attempt.** The first was stopped by pid: the machine had gone to a load average of 71
on 24 cores with six rounds building at once, which is the condition `doc/environment.md` says a
corpus walk is never run under, and `--test gate` carries RFC 0002 section 12's perf floor besides.
Run when it came back down, all eight pass:

| walk | test | bounded |
|---|---|---|
| `writer_corpus` | ok, 13.07 s | exit 0 after 58 s, peak 1.50 GiB |
| `split_corpus` | ok, 120.64 s | exit 0 after 157 s, peak 3.11 GiB |
| `merge_corpus` | ok, 201.12 s | exit 0 after 366 s, peak 3.04 GiB |
| `pages_corpus` | ok, 291.32 s | exit 0 after 332 s, peak 3.33 GiB |
| `optimize_corpus` | ok, 62.94 s | exit 0 after 141 s, peak 3.22 GiB |
| `archive_corpus` | ok, 28.75 s | exit 0 after 86 s, peak 1.66 GiB |
| `foreign_corpus` | ok, 214.38 s | exit 0 after 251 s, peak 6.70 GiB |
| `gate` | ok, 2.96 s | exit 0 after 3 s, peak 0.48 GiB |

So the refusal costs no corpus document a file it used to get. That was the expectation and the
reason for it — `optimize`, `split`, `merge` and `archive/rewrite` all set their root from
`catalog_of`, and `optimize`'s own `Refusal::Reconstructed` already declines the documents this
refusal could reach — but an expectation is not a run, and `foreign_corpus` is the one that asks
qpdf, poppler and mupdf rather than asking ourselves.

## The shared worktree

`cargo test -p conformance` fails on this tree for a reason that is not this round's: session 1010's
`SOURCE_ROOTS` fix reaches `raster/crates/` for the first time, and the 1 884 citations ADR 1024 §4
counted are now checked and failing. The ledger's own two tests
(`the_ledger_agrees_with_the_standard_and_with_the_tree`,
`the_ledgers_own_prose_names_clauses_and_tables_that_exist`) pass, which is the half this round's
edit could have broken.
