# 985 — The merge of 979–984, and a signature that lied

Date: 2026-09-12. ADR: 1006. Rounds merged: 979 (transparency, ADR 1000), 980 (colour, ADR 1001),
981 (fonts, ADR 1002), 982 (the validator's Annex B.1 reading, ADR 1003), 983 (the errata and the
lexer's silent salvage, ADR 1004). Round 984, the owner's direction review, was terminated by the
session limit before it wrote anything and is relaunched.

Files: `crates/pdf-transform/src/archive/mod.rs`, `crates/pdf-model/src/content/{path,text,transparency}.rs`
(979's two callers switched from its patches, its dead wrapper removed), `crates/pdf-model/tests/text_state.rs`
(the blending-glyph test rewritten to the two constructions), `crates/pdf-model/tests/oracle.rs`
(`AMBIGUOUS_MARKUP_APPEARANCES_AND_OPEN_POPUPS`), `crates/pdf-model/src/{colour,icc}.rs`,
`crates/pdf-model/src/content/colour.rs`, `crates/pdf-model/tests/colour_paths.rs` (980's four lint
sites, left when the limit hit), `doc/conformance/ledger.toml` (§9.3.8's test name),
`doc/habits/{tests-gates-and-reports,the-ledger-and-claims-about-this-tree}.md`,
`doc/traps/pixels-and-rasterisers.md` (trap 40), `doc/HANDOVER.md`, the two histories 981 and 983
whose gate figures the limit cut off.

The session limit terminated 981 and 983 mid-sequence and 984 before it began. Every code round's
ADR and history were already on disk; the merge finished what they could not: switched 979's two
callers (it could not touch `text.rs`, which was 981's), rewrote the one test whose premise 979 had
superseded — a Multiply-blended text object no longer *reports*, it takes the mode to the group's
`Do`, and a `Darken` over two colours takes §11.4.6's own backdrop — and wrote the oracle diagnosis
979 said the switch would need: `issue14438.pdf` page 1, twelve markup annotations with appearance
streams and twelve popups without, on which `ghostscript` draws the open popups with artwork
§12.5.6.14 says a popup does not have, `poppler` rejects the `Ink` on its own reading, and `hayro`
paints the highlight opaque. Every annotation on that page has a `/Rect`; the "Rect-less Ink" in
979's report was poppler's log, not the file.

Then the full §2 sequence, run alone: green everywhere but `archive_corpus`, which stopped on the
first signed document. ADR 1006 is what that was: 982's new predicate had caught the converter
**re-serialising an already-conforming document and carrying its signature into bytes it no longer
covered** — a file that lied, written that way since the verb existed, while
`doc/pdf-a-conversion-limits.md` section 3.6 had said for ninety sessions that a converted document
carries no signature and that the loss is asked for loudly. A source that conforms is copied byte for
byte now; a non-conforming signed one is refused with the regression named; section 3.6's report is
owed to the next converter round.
