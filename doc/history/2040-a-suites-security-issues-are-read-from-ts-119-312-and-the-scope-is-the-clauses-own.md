# 2040 — A suite's security issues are read from TS 119 312, and the scope is the clause's own

Session 2040, a single ledger round, the signatures lane (`doc/questions/A856` and `A855`,
`doc/todo/77` item 3; §12.8.3.4.5). ADRs 2896 and 2897; no question.

**The slice, and why.** The whole of both answers: A855's scope text is two strings, and A856's
tables are six short ones, so neither had to wait for the other. Tables 3.1 to 3.3 (ML-DSA, SLH-DSA,
hybrids) are not held: no such scheme is computed here, so h) never reaches one.

**The premise.** Held: `SecurityRules::NoneHeld` was the only h) answer and `notes` paraphrased a).
The owner had answered both, so nothing was blocked; no `A*` after A856 cites either.

**Built.** `verdict::applicability::ts_119_312`: TS 119 312 V2.1.1's Tables 1, 2, 3, 4 (25 rows), 6
and 7, cell by cell, clause 3.4's classes, and `Assessment::of` — each table asked, clause 7.3's equal
lengths, Table 7's `q` from the signer's certificate, every issue naming its rule, a phase-out compared
with the best signature time and the validation instant. `Report::security` carries it;
`SecurityRules` is gone. `Scope::heading` and `Scope::text` carry a) as printed. `notes` prints both.
Readings (P-521's lengths, Table 4's internal hashes, the PSS row by digest) are in ADR 2896.

**Tests.** 14 in `ts_119_312/tests.rs` (five transcriptions, nine behaviours, one member differing
each), two in `applicability/tests.rs`, one in `notes`. **Plants**: three (Table 6's legacy cell made
`R`, clause 7.3's `!=` made `>`, a full stop added to a)'s sentence): 7 tests failed; reversed, pass.

**Gates.** fmt, clippy `-D warnings --all-targets` (`pdf-signature`, `viewer-core`): 0. nextest of
the two: 1171 of 1171, 7 skipped. Rustdoc `-D warnings --document-private-items`: 0. Fuzz `cargo
check --bins`: 0. `viewer-ffi`, `viewer-host`, `pdf-transform` check: 0. Conformance after `git add
-N`: 0, 474 passed. Tier 2: `corpus` 101, 974 documents, 61 incomplete, and `raster_golden` 101, held 951,
moved 23, both as at HEAD; `selection_census` 0 (974); `awkward_classes` 0 (none killed);
`accessibility_census` 101, then 0 (1021): its whole-population count of untagged pages answering
honestly read 901 against 900, the placed TS 119 312 being one more untagged document, so the floor
is 901 and the seven ETSI texts it counts are named in `RATCHETED_SPECIFICATIONS`. `state.sh --round
2040 docs` 0.

**Habit, and what is left.** Read a table's notation clause before its rows: clause 3.4 makes a bare
`L` a date (`L[2034+]`), which no row says. §12.8.3.4.5 stays `implemented`; `doc/todo/77` item 3 is
now MGF1's hash for the PSS row, clause 9's key lifetimes, and national rules (none held).
