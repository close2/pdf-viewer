# 1771 — §13.2's family is read into the ledger, and the seventh sweep reads `out-of-scope` rows

Session 1771, slot 1 of batch 117, the ledger slot (ADR 2370); ADR 2376; ADR 2377 unused; no question.
I worked in the shared worktree and in my scratch directory, `scratchpad/r1771/`.

**Rows.** The twenty-one §13.2 rows review 1768 named, read sentence by sentence in `doc/md/` against
`pdf_model::media`, all from `out-of-scope`: to `implemented` §13.2.3, §13.2.3.1, §13.2.3.2, §13.2.3.3,
§13.2.4.1, §13.2.5, §13.2.6, §13.2.6.1, §13.2.7.1, §13.2.7.2.2, §13.2.7.4.2, §13.2.7.4.3; to `partial`
§13.2.4.3, §13.2.7.2.1, §13.2.7.3, §13.2.7.4.1, §13.2.7.4.4, and as their aggregates §13.2.4, §13.2.7,
§13.2.7.2, §13.2.7.4 with §13.2 itself. §13.2.1, §13.2.6.2, §13.2.6.3 and §13.2.7.5 stay on the entry.
§10.6.1–§10.6.4 carry RFC 0009 section 2.4's flipping route (A254 question 8); no status moved there.

**The premise that did not hold.** Review 1768 expected the in-scope half executed whole; two verdicts
contradict the clauses, each probed by a fixture before the rows were written. §13.2.4.3: "For media
types where an offset makes no sense (such as JPEG images), B and E shall be ignored, with no effect on
viability" — `Walk::clip` makes an image clip's MH offset non-viable. §13.2.7.2.1: a non-viable media
player info dictionary "shall be treated as if it were not present in its original array" — `players`
reads `/MU` as written, so an `/MU` of non-viable entries still refuses a clip with a content type.
`media.rs` is slot 4's, so both are reported, not fixed. The hypothesis (15–21 `implemented`) did not
hold: twelve.

**Evidence.** `crates/pdf-model/tests/media_viability.rs`, eight tests, one per subclause the rows cite.

**Instrument.** `--bin inapplicable` reads `SWEPT` (`inapplicable`, `out-of-scope`) and states each row's
own clause number as `Kind::Citation`, counted first (trap 268). On HEAD's ledger before the moves: 118
rows, 31 cited, ten of them among the twenty-one; the vocabulary alone names none of the twenty-one but
by the template's shared words. After: 91 rows, 15 cited, none of the twenty-one. The planted rows are
two unit tests, and reverting `SWEPT` to one status fails both.

**Gates.** `cargo fmt --check -p conformance` exit 0, `-p pdf-model` exit 0; `cargo clippy -p conformance
--all-targets -- -D warnings` exit 0; `cargo clippy -p pdf-model --test media_viability -- -D warnings` exit
0; `cargo test -p conformance --lib inapplicable` 12 passed; `cargo test -p pdf-model --test
media_viability` 8 passed, `--lib media` 16 passed; `--bin inapplicable`, `--bin cited`, `--bin
quotations`, `--bin overstated` exit 0; `tools/state.sh --round 1771 ledger` exit 0; `cargo test -p
conformance --no-fail-fast` exit 101 on the first run with four reds, none mine (§14.8's aggregate and
§14.8.5.4.4's map placement, slot 5's; `media/deprecated.rs:45`'s quotation, slot 4's; a sibling's record
budget), and exit 0 on the rerun after this record, 69 test binaries passing.
