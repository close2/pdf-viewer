# 1821 — The ranking re-run with the pack named, and a bystander font's descriptor

Slot 6 of batch one hundred and twenty-two, 2026-10-27, an archive slot (Sonnet, high); ADRs 2476
and 2477 and Q687 used. I worked in `scratchpad/r1821/`. No ledger status moves (ISO 19005 has no
ledger rows).
**Premise** held: the ranking re-runs on HEAD, with and without `QUORRA_CJK_PACK` (a copy of the
owner's pack, digests held). pdf.js `as-if-printed` 2b, converted/conforming/refused/unreadable:
793/2/160/11 without the pack, 804/2/149/11 with it; part 4 796/0/159/11 → 807/0/148/11;
`doc/corpora` unchanged. **Hypothesis** (the largest carryable reason holds under ten documents) held
as a count and not as a remedy: the largest reason after the press profile's 29 is a font naming a
face outside the 14 with no descriptor and no `/Widths` (ten documents, alone in each). In an export,
opening `described::describe` to it freed eight at 2b and 4 and veraPDF 1.30.3 passed 14 of the 16
outputs (the one with no `/BaseFont` fails 6.2.11.2-3), but the widths would be this viewer's
convention (Arial as Helvetica "in practice"), which A48 forbids filling in. **Not built**: ADR 2477,
Q687 asks. The other rows: eight `.notdef`-beside-descriptor documents all stop on another reason; the
DeviceN colourants and the headers would invent a tint transform and a version.
**Built (the fallback, ADR 2476).** Table 120's `/FontName` must equal the `/BaseFont` of every font
that refers to the descriptor; `issue13343`'s unrendered third descendant kept the object the pack
route renamed. `fonts::bystanders::keep_their_own_descriptors` runs once after every font is in and
points each font dictionary that names a renamed descriptor, and that the conversion gave neither name
nor descriptor, at a copy of the producer's descriptor (embedding nothing; no page renders it). Both
the pack and re-keying routes are covered. `issue13343` at 2b and 4: `/Ryumin-Light` over a
`/FontName /Ryumin-Light`, veraPDF PASS. **Proof**: two tests, each failing with the call disabled
(Table 120 assertion) and passing with it; the census with the pack is line for line the pre-change one.
**Found, not fixed**: the refusal sentence for the first row says "not built yet" of a decision (Q687
recommends the fence's wording); a bystander written inside another dictionary keeps the renamed
descriptor (no corpus document has one).
**Gates.** In an export of HEAD and my hunks: `cargo fmt --check -p pdf-transform` exit 0;
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform --all-targets` exit 0; rustdoc `-D warnings
--document-private-items` exit 0; `cargo test --profile gates -p pdf-transform --no-fail-fast` exit
0, 736 passed, 0 failed; with the pack `--test archive cjk_pack` 6 passed, `rekeyed` 4 passed;
`archive_corpus --ignored` with the pack exit 0, 7 passed; `cross_check --ignored` exit 0, 13 560
compared, 0 disagreements; census `--tree 6` exit 0 before and after, identical; `tools/state.sh
--round 1821 archive` exit 0; `cargo tree -i pdf-transform --edges all` reaches no render crate, so
against HEAD's six arms nothing moves; `cargo test -p conformance --no-fail-fast` exit 0 (its first
run failed `every_quotation_is_the_standards_own_words` on my module comment, fixed). 6365 s. **Validation** (the orchestrator): the Opus validator's verdict was PASS — the A48 call, the Table 120 quotation, the plant (both new tests red without the call), clippy, fmt and rustdoc all held; two comment notes were taken at the merge.
