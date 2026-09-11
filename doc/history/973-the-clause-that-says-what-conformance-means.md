# 973 — The clause that says what conformance means

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `8eac6aa2`. Stream: general
improvement — the slot kept open so that a focused push never takes every round. Four sibling
rounds were live in the same working tree throughout (969 in `pdf-font`/`pdf-model`/`render-*`,
970 in `pdf-archive`, 971 in `pdf-transform`, 972 in the instruction documents).

## What the round chose, and why over the alternatives

The brief offered `doc/todo/` items in the free crates, the normative annexes, and the method that
had paid three rounds running: *a constant, a table, or a claim in this tree, checked against a
datum the tree already carries*. It also asked one question — **what other instrument in this tree
takes its own output as its population?**

Annex O was checked first and is closed (`tools/state.sh annex-o` prints every parameter carried
out and none reported). The ledger's own population was the next thing to ask about, and it
answered the question directly.

Three rejected on the way, each for a reason worth recording:

- **The markdown conversion's headings against the standard's outline.** The PDF in `doc/` carries
  a full-depth outline, 943 numbered entries; every one of them is a heading in `doc/md/`. Nothing
  lost, so nothing to report beyond this sentence.
- **The printed table of contents.** It lists two levels only — 98 entries, where the ledger's rows
  are subclauses — so it cannot rank a missing subclause. Session 965's method does not transfer to this
  standard at this depth.
- **A check reading the tree's citations against the ledger's rows.** Written, run, and removed:
  see below.

## What it found

**Clause 6 — *Conformance* — had no row in the conformance ledger**, and it is the only clause of
ISO 32000-2 that says what conformance means for a *program*. It states eleven `shall`s, and
`CLAUDE.md`'s own *what done means* is written around three of them (§6.3.2.2's obligations on a
processor that renders a page). This tree cites §6.3.2.2 forty-seven times across `pdf-font`,
`pdf-model`, `viewer-core` and their tests.

**Nothing could see it, and the reason is the shape the brief asked about.** The population was
the constant `TECHNICAL_CLAUSES: 7..=14`; `MissingRow` and `CitedButUnreviewed` both walk it, so a
clause outside the constant had no row and having no row was exactly what made it invisible.

**A second finding came out of reading the new rows' quotations back.** Twenty-three Annex F rows
attributed to §F.1, in quotation marks, the phrase *"shall be a conforming file"* — which occurs
nowhere in ISO 32000-2, and whose root word `conform` occurs nowhere in that standard's Annex F at
all. It is ISO 32000-1's vocabulary. All twenty-three now quote F.1's own sentence, which says the
thing they wanted better.

## What changed

- **Eight new ledger rows**, each read against the code: §6.1, §6.2, §6.3, §6.3.1, §6.3.2,
  §6.3.2.1, §6.3.2.2, §6.3.2.3. Six `implemented`, one `writer-side` (§6.2, whose `shall` is on a
  *file*), one `partial` — §6.3.2.1, for the security `shall` whose debt §7.6.4.1's row already
  carries out loud.
- **The population is a checked claim rather than a constant.** `NORMATIVE_CLAUSES`,
  `NORMATIVE_ANNEXES`, `INFORMATIVE_ANNEXES` and `EXCLUDED_CLAUSES`, with `check` counting `shall`
  under every clause and annex of the standard and failing the conformance gate on one that is in
  none of them. Five unit tests, calibrated per trap 13 — the finding fires on the defect and does
  not fire on the same clause stating nothing.
- **Twenty-three Annex F rows** re-quoted.
- **`doc/PLAN.md` §5a and `doc/ledger-and-claims.md`** say the population is nine clauses and eight
  annexes, and that what decides it is the standard.

ADR 0984 has the clause reading, the four lists' argument, the two things this round deliberately
did not do, and the live defect it reports rather than fixes.

## The live defect this round reports and does not fix

`CLAUDE.md` and `doc/todo/02` both state that `§N` means ISO 32000-2 and nothing else and that
`cargo test -p conformance` enforces it. **For one shape it does not.**
`citation::ForeignCitation` catches a document named *immediately* before the `§`, so `§5a`,
`§3a`, `` `doc/todo/02` §2 `` and `RFC 0002 §6.1` all parse as ISO 32000-2 citations — of clauses
1, 2, 3 and 5, which exist, so `every_citation_names_a_clause_that_exists` passes them. The
citation scan sees roughly a hundred and thirty of these. This is why the citations-against-rows
check was removed rather than kept: it fires on that noise. Whoever takes it owns a scanner change
and sites in five crates.

## Gates

Run on a quiet machine, after the last edit.

| line | result |
|---|---|
| `cargo fmt --all --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | silent, exit 0 |
| `cargo nextest run --workspace` | **4317 tests run: 4317 passed (1 slow), 35 skipped** |
| `cargo test --workspace --doc` | 36 suites, all ok, none failed |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | silent |
| `cargo test -p conformance` | **227 lib + 11 integration passed, 0 failed** (`ledger::tests` is 12, five of them new) |
| `cargo run -p conformance --bin ledger` | 883 rows, 0 new, **0 unreviewed** |
| `cargo run -p conformance --bin pointers` | ran; nothing in the two new documents is absent |
| `cargo run -p conformance --bin quotations` | ledger notes: **1913 verbatim, 5 diverging, 494 unrelated** — was 1889 / 5 / 517 before the Annex F re-quote |

Nothing this round touched can change a pixel — the whole diff is a checker, the ledger and prose
— so the corpus, oracle, quorra, transform and vfs walks were not run, which is
`doc/todo/02` §2's change→gate map for `tools/conformance`, `doc/conformance/ledger.toml` and
documents.

**One line failed and was fixed rather than reported**, because it was this round's:
`tools/spec-errata` filters an editor's note's clause numbers through the same constant, and the
workspace clippy line is what found it. It now reads `NORMATIVE_CLAUSES`, so a number whose first
component is 6 is recognised as a clause — which it is.

**One failure was a neighbour's and was not touched.** For part of the session
`doc/conformance/ledger.toml`'s §8.6.5.5 row had its `test` array split across two lines, which
`toml_subset` refuses (`line 1594: a single-line array`); the sibling closed it, and every line
above was run green afterwards.

## Files touched

- `tools/conformance/src/ledger.rs`
- `tools/conformance/src/bin/ledger.rs`
- `tools/conformance/tests/conformance.rs`
- `tools/spec-errata/src/lib.rs`
- `doc/conformance/ledger.toml` — rows §6.1, §6.2, §6.3, §6.3.1, §6.3.2, §6.3.2.1, §6.3.2.2,
  §6.3.2.3 added; the notes of §F, §F.1–§F.4.7 re-quoted; the header comment
- `doc/PLAN.md`
- `doc/ledger-and-claims.md`
- `doc/adr/0984-the-clause-that-says-what-conformance-means.md` — new
- this file
