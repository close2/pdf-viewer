# 1092 — An unsanctioned array destination is read, and the alignment is not a choice at all

Status: accepted. Session 1078.
Context: `crates/pdf-font/src/cmap.rs` (`CMap::take_chars`, `CMap::take_ranges`,
`CMap::insert_single`, `CMap::insert_range`).
Clauses: ISO 32000-2 §9.7.5.4 c), §9.7.6.2, §9.10.3.

## 1. What §9.7.5.4 asks of a reader, which is nothing

The clause opens "Embedded CMap files shall conform to the format documented in Adobe Technical
Note #5014, subject to these additional constraints", and its five lettered constraints are
constraints on that file. c) — "The beginbfchar and endbfchar shall not appear in a CMap that is
used as the Encoding entry of a Type 0 font" — therefore binds whoever writes the CMap, and places
no requirement on whoever reads one. The ledger row for this subclause had it as two subclauses of
the standard disagreeing, because §9.7.6.2's account of the decoding algorithm names `beginbfchar`
among "the character code mappings". They do not disagree: one says what a conforming file holds,
the other says what a processor does with the file it is given, and the second is total by
construction. That reading is what closes §9.7.5.4, and with it §9.7.5 and §9.7, whose notes both
said this row was the whole of what they owed.

## 2. The array form, which the standard describes once and sanctions elsewhere

§9.10.3 writes the syntax down — "Consecutive codes starting with srcCode1 and ending with
srcCode2 shall be mapped to the destination strings in the array starting with dstString1 and
ending with dstStringm ." — and grants it to "the CMaps used for the ToUnicode entry". An
`Encoding` CMap writing `<lo> <hi> [<d0> <d1> …]` is outside that grant, and the standard says
nothing further about it.

**The choice is to read it, one selector per consecutive code.** Three sentences leave no other
reading: the form's meaning is stated for this operator in the one place the standard describes
it; §9.7.6.2 makes an `Encoding` CMap's destination a CID rather than a Unicode string; and
§9.7.6.2 makes the lookup itself a `shall`, so a mapping the file states and this reader discards
becomes §9.7.6.3's CID 0 on the page. Dropping it would be defensible — the syntax is
unsanctioned here — but it would spend a mark the producer asked for to make a point about a
constraint addressed to somebody else. This is the decision a later round should not re-open.

## 3. What is not a choice

The two section readers stepped three tokens and two tokens at a time. A three-token stride walks
*into* an array — `<0010> <0012> [` is one stride, and the next takes `<0041> <0042> <0043>` for an
entry, mapping codes the file never mentions and losing the entry that really follows. That is a
fabricated mapping, which §9.7.6.2's lookup then finds; it is the failure this project rates worst,
a confident wrong answer with nothing said. So both readers now step by operand and resynchronise on
the next token, and the array arm runs to its matching `]` whatever it holds. No corpus document is
known to write one: `raster_golden` and `text_extraction` are unmoved by the change, and the
witnesses are the three fixtures in `cmap.rs`, each confirmed to fail with the previous reading put
back (trap 13). That is trap 8 stated rather than hidden — the fix is owed by the clause and not by
a document.
