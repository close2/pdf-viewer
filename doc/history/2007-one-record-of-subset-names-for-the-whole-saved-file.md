# 2007 — One record of subset names for the whole saved file

Session 2007, a single ledger round, the ledger lane. Rows 7.4.9, 9.9.2, 9.9 and 7.10.5.3; owner answer A825;
ADRs 2832 and 2833.

**The slice, and why.** The brief's order: A825's Owes, then §9.9.2's file-wide subset tags, then §7.10.5.3's
run-time fault reports as far as the wall budget went. The first two are built; the third is set out, unbuilt.

**The premise.** Held for all three: unbuilt, effort only, no owner word needed (no Q838). One part did not hold
as briefed: five of the six `for_a_file` calls run inside the incremental update's one `Update`, and the sixth,
`appearance::for_annotation`, is called only by the archive conversion, which this lane may not touch.

**A825.** T.801's whole text for `EnumCS` 21 is one line of Annex M, "ROMM-RGB: As defined by PIMA 7666.". §7.4.9's
note now records the declined purchase, that the pinned neutrals prove only that the codec's profile keeps equal
components neutral and a chromatic sample chromatic, and that the primaries, white point and encoding, so which
grey and whether any colour is right, are what only PIMA 7666 could fix.

**Built (§9.9.2).** `variable_text::SubsetNames`, one per written file: a subset meeting a name another subset of
the write took with other contents, or a subset-tagged `/BaseFont` or `/FontName` the file already states (read
once, at the first subset), moves to the next tag. `Update` holds one for the save, through `for_saving`,
`toggling_states`, added and retyped appearances; `appearance::for_annotation_in` takes a caller's record for the
archive lane to adopt. `for_a_file` stays as the owner's audit test's single-dictionary form, test-only.

**Tests.** `variable_text_subset_names.rs` (four: the audit's colliding pair through one record, with a
record-per-stream control that collides; one subset twice keeps one name; a stated name is passed over; the tag's
shape) and `tests/subset_names_in_one_save.rs` (a save into a file already stating its DejaVu subset's name gives
another; the first save is the control). **Plant**: a name never taken fails four of six, the audit's included.

**Gates.** `cargo fmt --check -p pdf-model`: exit 0; clippy `-D warnings --all-targets` on `pdf-model`: exit 0
after two lints fixed; nextest of `pdf-model`: 2627 of 2627 passed; `cd fuzz && cargo check --bins`: exit 0;
conformance after `git add -N`: exit 101 on `adr_status` and a table name (both fixed), then exit 0; `state.sh docs`:
exit 0. Tier 2: `corpus` and `golden` exit 101 each, 61 incomplete and 23 first pages moved, the same 23 by name as
round 2000's walk of HEAD's tree, so nothing moved. `script_corpus` not owed, no viewer crate touched.

**Left of the rows.** 9.9.2 and 9.9 stay `partial`: the archive conversion's writers do not share one record, and
FDF-imported appearances keep their producer's names unchecked. 7.10.5.3 stays `implemented` with its unreported
callers named and ADR 2833's construction unbuilt (faults copied on clone, no list of a page's functions, the
GPU's `device_program` needing a fault word read back or a CPU fallback); 7.4.9 stays `partial` on ADR 2793's
reasons. Left for later: Table 196's mixed order, Table 197's `/PV` and `/PI`, FDF templates, prose 30 and 52, and
A828's XFA measurement.
