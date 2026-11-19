# 3086 — `pdf-model`'s integration tests are nine subject suites and twelve walks

Session 2096. Status: **accepted** (the owner's A907, option 2). It cites no clause and moves no ledger
row: no test file moved, so every `test` site naming `crates/pdf-model/tests/<file>.rs::<test>` is
the path it was. Context: Q907, A907, ADR 3036 (the `render-cpu` pilot and its method), ADR 2912
(linked programs are most of a build directory).

## 1. What changed

`crates/pdf-model/tests/` held 350 files, each a test binary of its own. Now:

- **Twelve stay binaries of their own**, under their own names: `actions`, `corpus`, `dates`,
  `drawn_while_read`, `fixed_documents`, `jpeg2000`, `oracle`, `raster_golden`, `save_round_trip`,
  `script_corpus`, `text_extraction` and `xmp` — every `pdf-model` test the tier-2 and tier-3 walks
  name by `--test` in `doc/todo/02` §2, `tools/state.sh` and `tools/batch.sh`. So no gate's command,
  no walk's log name and `raster_golden.tsv`'s path change.
- **The other 338 are modules of nine suites**, one root file each, named by subject:

| binary | files | lines | what it holds |
|---|---|---|---|
| `syntax_suite` | 30 | 8 477 | clause 7: objects, trees, the page tree, damaged files, budgets, permissions, §7.5.6's save |
| `content_suite` | 47 | 17 449 | clause 8's drawing: operators, graphics state, paths, clipping, shadings, patterns, §8.11 |
| `colour_suite` | 29 | 11 287 | §8.6 and clause 10: colour spaces, ICC, spot colourants, overprint, transfer, output intents |
| `transparency_suite` | 19 | 10 637 | clause 11: soft masks, groups, blending, matte |
| `images_suite` | 34 | 11 522 | §8.9 and the image filters: DCT, JPX, JBIG2, CCITT, masks, decode arrays |
| `text_suite` | 52 | 13 002 | clause 9: every font type, CMaps, text state, substitutes, the faces a save writes, extraction |
| `annotations_suite` | 34 | 9 712 | §12.5: every subtype's dictionary and appearance, XFDF |
| `forms_suite` | 50 | 19 801 | §12.7 and §12.8: fields, variable text, FDF, XFA, signatures, field scripts |
| `navigation_suite` | 43 | 9 344 | §12.1–12.4, §12.6, clause 14: outlines, links, labels, files, structure, accessibility |

**The grouping is the standard's.** A file was placed by the clause its own `//!` header cites
first, because that is what the file says it is testing and what a round editing that clause will
look for; 49 files straddle two subjects (an `indirect_*` audit names §7.3.10 first and a subtype
second, `links` is an annotation and a destination), and each went to its first clause. Clause 8
is split in four rather than kept whole because it is half the crate's tests: drawing, colour,
transparency and images are separate clauses or sub-clauses the ledger already reads apart, and a
suite of 60 000 lines would bring back §3's rebuild cost. The writers did not earn a suite: only
four files are about the bytes written, and three of them are clause 9's (a face or a subset name
a save writes), so they went to `text_suite` and `saving` to `syntax_suite`. Nine suites, at 8 500
to 19 800 lines each, are the "few" A907 asked for.

**How it is built.** `autotests = false` in the crate's `[package]` and a `[[test]]` for each of the
21 binaries. **No file moved.** A suite's root is a file in `tests/` beside its modules, so a `mod
outlines;` there resolves to `tests/outlines.rs` exactly as before; so `include_bytes!("fixtures/…")`
and `include_bytes!("../../../data/…")`, which are read from the including file's own directory,
`#[path]` helpers (`restrictions.rs`'s `support/corpus_passwords.rs`, `icc_paint_context.rs`'s four
parts), and every `CARGO_MANIFEST_DIR` path resolve as they did. ADR 3036's layout — a directory
`tests/all/` with the files moved into it — would have broken the owner's audit files that include
a fixture by a relative path (nineteen of them), which move unedited (A868, A875), or kept them out as
binaries of their own. No module sets the environment or the working directory; one keeps statics
of its own (`machine_cff_face_written`), which are the module's in a shared process as they were the
binary's.

**What turning discovery off costs, and the check that pays it.** With `autotests = false`, a new
file dropped into `tests/` is compiled by nothing and run by nothing, and `cargo test`, `cargo
nextest run` and `clippy --all-targets` all exit 0 over it. `tools/conformance/tests/test_targets.rs`
fails on any `tests/*.rs` of a crate with discovery off that no `[[test]]` root names by a `mod`
line (a planted orphan failed it). And `tools/conformance/tests/commands.rs` now reads such a
crate's targets from its `[[test]]` table alone, where it had counted every file stem under `tests/`
as a target — which would have kept passing a stale `--test silent_fonts` in a document.

**Names and filters.** nextest's `pdf-model::<file>` becomes `pdf-model::<subject>_suite` and the
test `<test>` becomes `<file>::<test>`. `cargo test -p pdf-model --test <file>` becomes `--test
<subject>_suite <file>::`, and nextest's filter is `-E 'test(/^<file>::/)'`; each root's `//!`
header says so. Five live references followed: `doc/gwg-output-suite.md`, `doc/todo/03`, trap 1 of
`doc/traps/pixels-and-rasterisers.md`, and the `--ignored` commands in `restrictions.rs` and
`transparency_groups.rs`.

**The count is equal**: `cargo nextest list -p pdf-model` names 2 896 tests before and 2 896 after,
and the sorted lists are identical once a suite's `<file>::` is read as the binary it was; nextest's
summary is `2874 tests run: 2874 passed (2 slow), 22 skipped` on both trees.

## 2. The numbers

ADR 3036's method: `CARGO_INCREMENTAL=0`, `RUSTC_WRAPPER=""`, a private target directory removed
before each clean build, `cargo nextest run -p pdf-model --no-run` under `tools/bounded.sh --tree 24`,
processor time the children's user plus system; the one-file rebuild touches `tests/outlines.rs`
(`navigation_suite`, 9 344 lines). The before tree is a copy of this one with the change reversed.
Four lanes and another user were building throughout; load is the one-minute average before each.

The first sitting ran the two layouts' scripts overlapping (the before tree's second build beside the
after tree's first); the second interleaved them, before then after, twice, and then measured each
layout's rebuilds with incremental compilation on (`CARGO_INCREMENTAL=1`, the edit loop a round may
use), after one incremental build of its own. The second file touched is `tests/variable_text.rs`, in
`forms_suite`, the largest suite (19 801 lines).

| | load | clean build wall s | clean build CPU s | directory MiB | test binaries |
|---|---|---|---|---|---|
| before, first sitting | 119, 153 | 1798.6, 452.8 | 3154, 3178 | 11 646 | 350 |
| after, first sitting | 206, 164 | 314.1, 660.5 | 2783, 2718 | 3 614 | 21 |
| before, interleaved | 49, 64 | 634.1, 236.6 | 2794, 2865 | 11 646 | 350 |
| after, interleaved | 152, 104 | 188.8, 151.1 | 2331, 2177 | 3 614 | 21 |
| before, incremental on | 57 | 252.4 | 3041 | 14 365 | 350 |
| after, incremental on | 81 | 367.3 | 2452 | 5 801 | 21 |

| one file touched, then rebuilt | before wall / CPU s | after wall / CPU s |
|---|---|---|
| `outlines.rs`, incremental off | 113.3 / 31.5, 16.0 / 18.6, 16.1 / 20.7, 10.1 / 17.3, 10.0 / 12.9 | 15.8 / 32.9, 12.3 / 22.1, 10.0 / 25.0, 11.2 / 31.3, 11.2 / 29.9 |
| `variable_text.rs`, incremental off | 15.7 / 25.1, 10.2 / 18.2 | 20.3 / 66.3, 18.1 / 51.9 |
| `outlines.rs`, incremental on | 10.4 / 20.1, 11.6 / 18.2, 9.5 / 15.9 | 6.9 / 7.1, 4.5 / 3.9, 4.6 / 4.1 |
| `variable_text.rs`, incremental on | 10.9 / 21.4, 11.4 / 20.1, 6.8 / 14.1 | 5.6 / 5.5, 4.5 / 4.7, 4.5 / 4.7 |

One nextest run of each tree (`xvfb-run -a cargo nextest run -p pdf-model`, after `cargo build -p
pdf-sandbox --bins`) took 222.6 s before at a load of 240 and 117.4 s after at 76; the loads differ
too much for the pair to say anything but that the run is not slower.

What the numbers say, medians over the four clean builds of each:

- **The directory falls by 69 %**, 11 646 to 3 614 MiB, identical at every load: 329 linked copies of
  the crate's graph are gone. With incremental compilation on it falls 14 365 to 5 801 MiB. Most of
  Q876's 12 GiB for this crate's tests was linked programs, as ADR 2912 predicted.
- **A clean build's processor time falls by about a sixth** (3 009 to 2 524 s), less than
  `render-cpu`'s quarter because `pdf-model`'s tests are more of the work and the links less; its wall
  time roughly halves (544 to 252 s), which at these loads is mostly the 350 links no longer queuing
  behind each other.
- **A one-file rebuild with incremental off costs the same wall** (16.0 to 11.2 s for `outlines.rs`,
  13 to 19 s for the largest suite's file) and more processor time (18.6 to 29.9 s; 22 to 59 s for
  `forms_suite`): the edited file's whole suite recompiles, 9 000 to 20 000 lines, where one file did,
  but one link replaces one link and the link is most of the wall. This is the cost A907's option 2
  was chosen to bound: one binary would recompile 111 000 lines on every edit.
- **With incremental on, the rebuild is about twice as fast after** (10.7 to 4.6 s, 19 to 5 s of
  processor time, over both files): the query cache keeps the suite's unchanged modules, so the
  compile is the touched module's, and the remainder is one link either way. Why the before tree's
  link of one small binary costs more here than the after tree's of one large one was not measured.

## 3. What it costs

- **A gate's rebuild after a test edit compiles the edited file's suite** (§2): seconds of processor
  time more under `CARGO_INCREMENTAL=0`, the same wall; an incremental edit loop is faster than before.
- **A new test file is compiled only when a suite names it.** That is the price of `autotests = false`,
  and `test_targets.rs` is what keeps it from being silent. A file meant to be a walk of its own gets a
  `[[test]]` beside the twelve.
- **Failure isolation and parallelism are unchanged under nextest**, which runs each test in a
  process of its own whatever the binary. Under `cargo test` a test that aborts the process takes its
  suite with it, and a suite's tests are scheduled over one harness's threads.
- **A compile error in one module stops its suite**; the other twenty binaries still build. Clippy
  `--all-targets` reports every error either way.
- **A filter names the suite**: `--test text_suite silent_fonts::`, where it was `--test silent_fonts`.
  A document that still writes the old form fails `commands.rs`, which now reads the declared targets.
