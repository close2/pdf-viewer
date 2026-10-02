# 1497 — A row held only by walks owes a fixture, and keeps its status

Session 1331. Status: accepted. Code: `tools/conformance/src/ledger.rs` (`TestKind`,
`TestClassifier`, `Holding`, `Problem::OnlyWalks`), `tools/conformance/src/bin/ledger.rs` (the
`held by` lines), `tools/conformance/tests/conformance.rs` (`ONLY_WALKS_CEILING`). Fixtures:
`crates/pdf-syntax/tests/encryption_fixtures.rs`, `crates/pdf-model/tests/composite_font_fixtures.rs`,
`crates/pdf-model/tests/colour_paths.rs` (one test added).

## 1. The question

The checker verified that each `test =` entry names a function that exists. It never asked what
kind of function. Principle 5 wants a test's expected value derived from the clause; `CLAUDE.md`'s
two denominators keep the corpus for the robustness question. A row whose only tests are corpus
walks is held by the second instrument while claiming an answer to the first.

## 2. What the measurement found, against the brief's premise

The brief expected rows held only by `#[ignore]`d walks. **There are none.** Of 3 129 entries, 30
are ignored walks and 2 are functions in `src/bin/`, and every row naming one also names a test
that runs. The class that does exist is different: a `#[test]` with no `#[ignore]` whose body reads
from `doc/pdf.js`, `doc/corpora/` or `doc/veraPDF-corpus`, directly or through a helper in its own
file. Each returns early, with a "skipped" line, when the checkout is absent. Its expected value
is often the clause's (the vertical-metrics test asserts Table 115's `/DW2` default), but on a
machine without the corpus it passes having read nothing. At the start of this session 35
`implemented` rows and 3 `partial` rows were held by nothing else: seventeen §7.6 encryption rows
plus §7.4.10, ten clause-9 composite-font rows, §8.6.4.2, §12.3.1, §12.3.2, four clause-14
tagged-PDF rows and §I.1.

## 3. The instrument

`TestKind` has six values: `Fixture`, `CorpusWitness`, `Walk` (ignored), `Census` (a program's
function), `File` (no `::`) and `Missing`. `TestClassifier` reads each file once. It does not parse
Rust. It relies on rustfmt's layout: a function's attributes are the lines directly above it, and
its body ends at the `}` with the signature's own indentation. That keeps the checker's single
dependency, `thiserror`. The corpus test grows to a fixed point over the functions of one file. Its
known blind spot: a corpus read through a helper in *another* file is missed unless this file's
text names a corpus root. A `Holding` is `Fixture` if any entry is a fixture, `Empty` if there are
none, and `OnlyWalks` otherwise. `bin/ledger` (and so `tools/state.sh ledger`) prints the three per
status.

An `implemented` row in `OnlyWalks` is reported by name as `Problem::OnlyWalks`. This is a finding,
not a failure. `the_ledger_agrees_with_the_standard_and_with_the_tree` admits these rows up to
`ONLY_WALKS_CEILING`, and the assertion is `==`, so the number cannot grow and must be lowered
when it falls. Unit tests calibrate the classifier on a planted file holding each kind, and
calibrate the problem both ways: a walk and a witness are a finding, and adding one fixture ends it.

## 4. How A100 bears

The owner's A100 concurred, in one sentence, with the recommendation of its question, and the reading
it ratified is that a requirement executed on request, through a control a host supplies, is executed.
The default and its argument go in the note, and `departed` is kept for a decision against the
requirement. Applied here, the presence of a corpus is a control in exactly that sense: the walk
and the witness run the code whenever it is present. So **none of these rows changes status**.
`implemented` stays true of the code. What the rows lacked was not execution but evidence a clause
can be read off — the second half of principle 5's sentence. That is why the instrument names
evidence, not status, and why the debt is a fixture added to the `test` list. Under A100, a row
moves to `partial` only if reading its clause finds a requirement the code does not execute, and
none of the clauses read this session (7.4.10, 7.6, 8.6.4.2, 9.7.4.3 and their neighbours) found one.

## 5. What was built

Encryption: revisions 2, 3, 4 (AESV2, including unencrypted metadata) and 6 are encrypted by the
test's own transcription of Algorithms 1, 1.A, 2, 2.B and 3 to 10. They are never encrypted by
`pdf_syntax`. The tests assert user/owner/wrong-password outcomes and the decrypted content and
`/Title`. They also cover §7.6.2's `/ID` and `/Encrypt` exceptions, `/StmF /Identity`, and §7.4.10's
`Crypt` filter naming `Identity`. Composite fonts are Type 0 dictionaries over the tree's
`LiberationSans-Regular.ttf` with an identity `/CIDToGIDMap`, so the glyph index *is* the CID. They
cover an embedded one-byte `CMap`, a UTF-8-shaped codespace, `90ms-RKSJ-H` by name, and §9.7.4.3's
EXAMPLES 1 to 3 encoded as written. DeviceGray: §8.6.4.2's levels, its EXAMPLE's `CS`/`SC` against
`G`, and the reset to 0.0. §12.3.1 and §12.3.2 name fixture unit tests already in `outline.rs`,
`thumbnail.rs` and `destination.rs`. Each new test was calibrated by a mutation that makes it fail.
Measured before and after with `cargo run -p conformance --bin ledger`: `implemented` went from
650 / 35 / 0 to 680 / 5 / 0, and `partial` from 8 / 3 / 0 to 10 / 1 / 0.

## 6. What it costs, and what is left

The encryption fixtures share a blind spot with the reader: a step both transcriptions misread the
same way passes. The producers' files in `tests/encryption.rs` stay in every row for that reason.
The five rows left (§14.8.2.5.2, §14.8.2.6, §14.8.2.6.2, §14.8.6, §I.1) and the one `partial` row
(§7.4.9) are listed in `doc/todo/65`'s eighth shape by command, not by count.
