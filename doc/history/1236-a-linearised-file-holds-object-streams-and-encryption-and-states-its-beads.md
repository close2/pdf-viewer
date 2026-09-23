# Session 1236 — a linearised file holds object streams and encryption, and states its beads

Batch thirty-six, parallel. Contract: F.3.1, F.3.5, F.3.7 (`partial`), then F.3 and F; ADR 1293
section 5's three residues. ADR 1309 is the design.

**F.3.1.** `optimize --linearize` packs §7.5.7's object streams run by run — part 4, the first
page's own objects, each item of part 6's trailing run and of part 8, each later page, each F.3.10
category — a carrier standing where its first member stood, so every hint table still counts whole
items and a carrier is one shared object group. Compressed objects take the highest numbers in
each section, the hint stream last; both sections are §7.5.8 streams (`/Index` over F.3.4's one
subsection, `/T` the main stream's offset). Without object streams the serializer's `Options::form`
decides, so a source with cross-reference streams now linearises into them too. The CLI's default
under `--linearize` is the ordinary default, generate.

**F.3.5.** `serialize_linearized_encrypted`: every object encrypted once under its final number
before the layout, the dictionary part 4's last item, the hint stream under one vector drawn
before the fixed point. Revision 6's key takes nothing from `/ID`, so the identifier stays the
digest. The refusal and its test are gone.

**F.3.7 (b).** Table 163 and Table 31 let a producer omit a later bead's `/T` and a page's `/B`,
so carrying them does not satisfy (b); `thread_beads` derives both from §12.4.3's chain. The
synthesised order (threads in `/Threads` order, then chain order) is a documented choice.

**Tests.** Every `tests/linearize.rs` fixture runs in both shapes; three new tests (F.3.1's
conditions, an encrypted file read back with its user password, two threads crossing page one).
`support::linearized::faults` now asks F.3.1's conditions and the stream form's `/T` and `/Prev`.
The first corpus run found the reader's own defect: `startxref` was read as text from a tail that
a binary cross-reference row reaches; it is searched as bytes now.

**Corpus (pdf.js, 974).** Both shapes: 1918 linearised and read back (959 with object streams,
87698 objects compressed), 1916 page ones bit-identical, 0 Annex F faults, 0 not idempotent, 2
refused (`Pages-tree-refs.pdf`, both shapes). **qpdf.** Reads both new shapes as linearised, and the encrypted one as `R = 6` AESv3 under the
password. One new warning — the hint stream, uncompressed, after compressed objects — is F.3.6's
own numbering; qpdf's writer numbers it before part 6, which F.3.6 does not allow. Annex stands.

**Ledger.** F, F.3, F.3.1, F.3.5, F.3.7 `implemented`; `doc/todo/65` loses their bullets and the
aggregates. Shared-writer hunks in `serialize.rs`: `Protected` and four methods `pub(crate)`, a new
`stream_with_iv`, `carrier_level` factored out of `flush`, and `header_version`, `row`, `deflate`,
`ObjectStreams::ceilings` made `pub(crate)`. The object-stream writer itself is unchanged.
