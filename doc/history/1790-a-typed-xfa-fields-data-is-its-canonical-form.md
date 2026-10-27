# 1790 — XFA's third round: a typed field's data in its canonical form

Session 1790, a clause round in batch 119; 2026-10-26. ADR 2414; ADR 2415 and Q656 unused.

**Contract.** Slot 3: RFC 0011 step 3, a numeric, decimal, float, date, time and date-time
field's `datasets` value in its canonical form; and A639's rewording of `CLAUDE.md`'s XFA entry.

**Premises.** *Two of the four hybrids carry a numeric or date field* did not hold. None has a
`numericEdit`, a `dateTimeEdit` or a picture; their `integer` values are check buttons' `items`.
All four carry a `localeSet` packet (`nl_NL`, `es_ES`), which is what the locale reader is built on.
*A value that does not parse stays unkept* held only in part. Chapter 4's Rule 2 (page 163) writes
text no edit picture matches as typed. Only the `validate` picture's format test (pages 162, 382)
gates the data, and that test is step 4, so such a field is named.

**Built.** Three modules in `pdf_model::xfa`. `canonical` recognises and writes chapter 24's date,
time, date-time and number forms. `picture` reads chapter 26's grammar for input parsing.
`locale` reads the `localeSet` packet and chooses the prevailing locale by page 1153's order. The
template now reads each field's value type, `locale` and `ui`/`bind`/`validate` pictures, and
`Datasets::keep` takes the form's locales. What is not done is named in `xfa_unkept`: a `bind`
picture, a `validate` picture, a time zone, an exponent, a week, a picture-less `dateTime`, and
non-numeric text in a `numericEdit`. `CLAUDE.md`'s XFA entry now splits the halves in A639's terms;
nothing else in it changed. Four inconsistencies in XFA 3.3 itself are in ADR 2414.

**Files.** New: `crates/pdf-model/src/xfa/{canonical,picture,locale}.rs`, ADR 2414, this record.
Edited: `crates/pdf-model/src/xfa.rs`, `crates/pdf-model/src/xfa/{datasets,template}.rs`,
`crates/pdf-model/src/view/writing.rs` (one call), `crates/pdf-model/tests/xfa_datasets.rs`,
`CLAUDE.md` (XFA entry), ledger K.2's note (`-U0`, no status move), RFC 0011 section 13,
`doc/state-of-play.md`. I worked in `scratchpad/r1790/`.

**Gates.** Tier 1: `rustfmt --check` on my files, exit 0; `cargo fmt --check -p pdf-model`, exit
0; clippy `-D warnings` `-p pdf-model --all-targets`, exit 0; `cargo nextest run -p pdf-model`,
exit 0, 2 260 passed; `--test xfa_datasets`, 7 passed, and the two new fixtures fail with the
canonicalisation planted out (trap 13); rustdoc `-D warnings` `-p pdf-model`, exit 0. Against HEAD
plus my paths, in an export: `save_round_trip`, exit 0, ratchet held; `writer_corpus`, exit 0.
`tools/state.sh --round 1790 docs`, exit 101, on `viewer-device`'s and `conformance`'s private
links and the toolkits, none mine. `cargo test -p conformance --no-fail-fast`, exit 101, 468
passed, 1 red and not mine (record 1789's length).

**Owed.** The `CLAUDE.md` edit meets RFC 0011 section 11's condition for Annex K's `partial`, a
move not this slot's. Step 4, validations, is next; its formatter also writes `bind` pictures.
