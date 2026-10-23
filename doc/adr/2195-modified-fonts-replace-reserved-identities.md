# 2195 — Modified fonts replace reserved identities

Session 1680, archive. Status: **accepted** at the merge that took it, `b9d05ee8` (line amended by ADR 2260). The round wrote “accepted; implementation under validation”.
Owner answer A550 accepts Q550's modified-Liberation recommendation.

The shipped Liberation license reserves its family name in Modified Versions.
Archive conversion can change horizontal or vertical metrics, and its CID route
can remove a TrueType cmap. These modifications now rename active OpenType name
identities from Liberation to QuorraText. The descriptorless route still refuses
metric changes. Unmodified programs remain byte-identical.

The font crate owns a narrow checked sfnt helper. It changes family, subfamily,
unique/full/PostScript, typographic, compatible full, CID findfont, WWS, and
variation-prefix identity records across platform/language encodings. Copyright,
license, attribution, and language-tag records are retained. Name records can
share storage: each changed identity receives appended string bytes and its own
offset. In-place replacement would corrupt a legal record sharing those bytes.
Malformed identity ranges or unrepresentable string offsets refuse conversion.
The existing sfnt writer repairs table checksums and head checkSumAdjustment.

The PDF's requested BaseFont and descriptor FontName remain paired. This helper
changes the embedded program's identity, not character selection, widths, outlines,
or the converter's existing face-substitution consent. ISO 32000-2 §9.6.3's
name-table derivation recommendation does not override those preservation rules.
Other embedded font identities are retained. Renaming an already renamed program
is byte-idempotent; it does not rewrite or strip orphaned name-table storage.

Focused witnesses cover every shipped Liberation style, untouched glyph tables,
whole-font checksum, shared identity/legal storage, malformed offsets, unchanged
copies, both metric-rewrite routes, and the composite cmap-removal route.
