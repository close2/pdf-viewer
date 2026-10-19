# 1577 — A short ICC profile is supplied or stated, and XMP is the 2005 edition

Slot 1 of batch ninety-two, 2026-10-19, an archive round. ADRs 1990, 1991; no question.

**Premises and hypotheses.** ADR 1979's held: `issue14814.pdf` lacks only `cprt`, `435321.pdf`'s
profiles do not parse. The XMP hypothesis did not hold, the wrong way round: ISO 19005-2's clause 2
does not list the XMP Specification at all; its one identification is Bibliography [20], the
September 2005 edition, which defines none of the four properties — three are in no edition
(Part 1 2012, Part 2 2022 read), `OriginalDocumentID` arrives in 2012 — and veraPDF reads the same
edition. The census's next `not-built-yet` site (2a, the role map) rested on a validator holding
part 2 to PDF 2.0's types alone: of 177 tagged corpus documents 38 named the row, 10 now.

**Built.** `pdf_archive::icc_tag_shortfall` (the row's reading, shared); `archive/icc.rs`: a profile
drawn through and short only of `desc`/`cprt` takes the operator's `description`/`copyright`
(`ProfileTextSupplied`, the edition's tag type, table grown and offsets moved); one not drawn
through — or damaged — is written as its Table 65 alternate (`ProfileAlternateWritten`), with the
output intent a device alternate needs prepared and written; mixed, `Default`-space, transform-short
and alternate-less ones refused by name (ADR 1990). `as-shown`, `as-if-printed`, `keep-everything`
state both texts. XMP (ADR 1991): `needs_a_description` makes an undefined name in a predefined
namespace an extension schema's to describe, by property; `xmp:Rating` back to the 2005 integer;
an `undeterminable = "discard"` cut now named in the report. Role map (ADR 1990 sections 6–7):
`archive/roles.rs`, `RoleMapSupplied` from the operator's `role-map`; the eleven Annex M PDF 1.7
types standard in the default namespace (`standard_structure_element`, `editions::SHIFTS` row).

**Pixels.** `issue14814.pdf` → 2b under `as-shown`: 2 of 2 pages byte-identical at 72 and 150 dpi,
veraPDF PASS (refused at HEAD). `102952.pdf` → 2b: veraPDF FAIL at HEAD, PASS now. Fixtures:
display lists identical through a CalRGB alternate and a supplied `cprt`, within 1/255 through
DeviceRGB under the sRGB intent the conversion adds.

**Gates.** rustfmt clean on every file touched. `RUSTFLAGS=-D warnings clippy -p pdf-archive -p
pdf-transform --all-targets`: exit 0. `nextest -p pdf-archive -p pdf-transform`: 813 passed, 0
failed. `cargo test -p conformance --no-fail-fast`: 440 passed, 0 failed. `tools/state.sh docs`:
exit 0. One `--tree 12` hold, 372 s: `t3-archive_corpus` exit 0 (2b 486 converted, 125 refused —
485/125 at HEAD; 4: 226/133), `t3-archive-val` exit 0, `cross_check` exit 0, veraPDF corpus census
exit 0 (2 908 of 2 908 opened), pdf.js and corpora census exit 0 (1 450 of 1 477; 32 unbuilt at 2b,
36 at 2a). One `--clock` hold, 191 s: `t3-foreign_corpus` exit 0 (203 of 974 in 134.4 s).
`--remedy-sites`: 2b 32 of 74 not built (33 of 75), 2a 36 of 79 (38 of 80).

**Unfinished.** The part 4 ICC row. `pdf_model`'s `Tree::standard_role` still reads a `Note` with
no `/NS` as non-standard against §14.8.6.1's default namespace (a sibling's crate). Duration: 8 000 s.
