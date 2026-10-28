# 2557 — A shown CID 0 is the CIDFont's own `.notdef`, and keeps the refusal

Status: implemented. Session 1861.
Context: ISO 19005-2 section 6.2.11.8, ISO 19005-4 section 6.2.10.9; ISO 32000-2 §9.7.4.2, §9.7.5 and
§9.7.6.3; ADR 2556, whose copy this narrows; A694 item 3.
Code: `crates/pdf-transform/src/archive/fonts/cid_notdef_glyph.rs` (`given`, `CID_NOT_GIVEN`, the module's
"Never CID 0" paragraph), `tests/archive/cid_notdef_glyph.rs`
(`cid_zero_under_the_identity_is_refused_and_no_file_is_written`).

## 1. The reading

ISO 32000-2 §9.7.4.2:

> Every CIDFont shall contain a glyph description for CID 0, which is analogous to the .notdef character name in simple
> fonts (see 9.7.6.3, "Handling undefined characters").

So in a CIDFont `.notdef` is a *character*, CID 0, and not a glyph index: ADR 2556's copy gives a CID on glyph 0 a glyph
of its own, and for every CID but 0 that is a CID that no longer selects `.notdef`. CID 0 stays the CIDFont's `.notdef`
whatever glyph a `/CIDToGIDMap` sends it to, so a text-showing operator that shows CID 0 still references it, which ISO
19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid. Under `/Identity` (stated, or absent, §9.7.4.2) CID 0
is the only CID that reaches glyph 0, so no font that states the identity is answered by the map at all.

**Evidence, not truth (principle 5):** veraPDF fails the copy's output for one corpus document at PDF/A-2b on 6.2.11.8
with the glyph it reports as CID 0, and passes the map-stream document whose CIDs on glyph 0 are 1 and above at both
targets. The agreement with §9.7.4.2 is what raises confidence; the clause decides.

## 2. Decision

`cid_notdef_glyph::given` refuses a font any of whose shown CIDs on glyph 0 is CID 0, with `CID_NOT_GIVEN` naming the
condition. Nothing else in ADR 2556 changes.

## 3. What CID 0 would take (not built)

The code that selects CID 0 is the font's to keep and the content stream's bytes are the producer's, so the route left
is the `Type0` font's `/Encoding`: an embedded CMap (§9.7.5) that sends each such code to a fresh CID past every CID the
font uses, with that CID given the copy through the restated map and its width written into `/W` at the width CID 0 had.
That restates a predefined CMap as an embedded one (ISO 19005-2 section 6.2.11.3.3's conditions on an embedded CMap's
`WMode` and on the CMaps it uses), adds a CID to the font's character collection, and moves what §9.10.2's text methods
read for the code unless a `ToUnicode` entry is written for it — three rows' worth of proof, and its own slot.

## 4. Found with it, not fixed

`pdf_archive`'s `no_notdef_glyph_shown` (`crates/pdf-archive/src/table/fonts.rs` ~1774) reads `.notdef` by glyph index,
so it passes a file whose map sends a shown CID 0 to another glyph; by §9.7.4.2 it should count a shown CID 0 as a
`.notdef` reference. Not changed here: it is the checker, and the census rests on it.
