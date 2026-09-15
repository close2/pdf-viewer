# 1078 — The constraint was on the file, and the stride was ours

Date: 2026-09-15. Branch: `batch-1074-1079`, worktree `/home/AI/pdf-viewer-rounds`, five siblings.
ADR: [1092](../adr/1092-an-unsanctioned-array-destination-is-read-not-dropped.md). Files:
`crates/pdf-font/src/cmap.rs`, `doc/conformance/ledger.toml`.

Clause 9's remaining `partial` rows are four — **§9.7**, **§9.7.5**, **§9.7.5.4**, **§9.8.3.3** — and
the first three are one debt written three times, both parents saying that §9.7.5.4's c) is all
they owe.

## Three rows closed by one sentence, and a defect found under it

§9.7.5.4 opens "Embedded CMap files shall conform to the format documented in Adobe Technical Note
#5014, subject to these additional constraints", so its five lettered items constrain the *file*.
c) — "The beginbfchar and endbfchar shall not appear in a CMap that is used as the Encoding entry of
a Type 0 font" — therefore asks a reader nothing, and §9.7.6.2, which is where a processor's
obligation is written, names `beginbfchar` among the character mappings a code is looked up in. The
row had held those two as *disagreeing* subclauses. They do not, and §9.7, §9.7.5 and §9.7.5.4 are
now all `implemented`.

**And reading c) properly found a defect of this reader's.** c) names `beginbfchar` and `endbfchar` and **not** the range operators beside them, so a `bfrange` in
an `Encoding` `CMap` is not prohibited at all — and §9.10.3 gives that operator an array form,
granted there to "the CMaps used for the ToUnicode entry". `CMap::take_ranges` read its section in
strides of three tokens, which walks *into* such an array and takes `<0041> <0042> <0043>` for the
next entry's bounds: a mapping for codes no file states, which §9.7.6.2's lookup then finds, with
nothing said, and the entry that really followed lost. Both readers now step by operand and
resynchronise on the next token, the array arm runs to its matching `]` whatever it holds, and the
array is read as one selector per consecutive code — ADR 1092's argued choice. Three fixtures in
`cmap.rs`, each confirmed to fail with the previous reading put back (trap 13): the array's
selectors, the entry after it — `Some(67)` for a code the file never mentions, under the old stride
— and a stray operand not costing the `cidchar` entries behind it. No corpus document writes such a
CMap, so the fixtures are the only witnesses; trap 8 stated rather than hidden. `bug920426.pdf`
still draws "Checkliste Service" (trap 1).

**§9.8.3.3 stays `partial`**: Table 123 separates `Rot`, `Ruby`, `Dingbats` and `Generic` by a glyph's *form*, or by nothing a Unicode value carries.

`raster_golden` held 974, moved 0; `pdf-model --test corpus` passed, every ratchet at its ceiling.
`conformance`, `text_extraction` and the oracle fail on siblings' in-flight work, not on this: with
this change reverted the text gate prints the identical 8266 / 11094 of 11131 / 503, the oracle names
the same one newly contradicted page, and the conformance report names only sibling files.
