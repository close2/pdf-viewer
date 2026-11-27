# Specification audit of ISO 19005-2 and ISO 19005-4 clause 6 (PDF/A-2 and PDF/A-4 files) — round 2282

The project's twenty-fourth specification audit, and the first of ISO 19005, made beside the other
lanes in the shape of the owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md))
and of audits 2128 to 2278 ([`2278-specification-audit-13.md`](2278-specification-audit-13.md) is
the one before). Baseline: main at `585a90aef`. Evidence:
[`2282-specification-audit-19005/`](2282-specification-audit-19005/) — `findings.json` (2038's
schema) and `expected-failures.json` (the failing test of each behaviour finding). The same
entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.
ISO 19005 is cited by section and paraphrased here, never quoted (`doc/third-party-data.md`).

## Method

1. Clause 6 of both parts (`doc/pdfa/ISO_19005-2_2011.md`, `ISO_19005-4_2020.md`), with clause 5
   and the normative annexes where clause 6 leans on them, read against `pdf_archive`'s table:
   268 rows in six tranches (197 checked, 45 a processor's, 25 unchecked with a reason, one
   outside validation), and `pdf_archive::coverage`, which carries every sentence of both parts to
   a row or to a reason it has none. The ISO 19005 ledger is that coverage table; `doc/conformance/
   ledger.toml` is ISO 32000-2's and carries no 19005 row. Errata: `doc/errata/pdfa.md` (every
   PDF/A issue with its verdict in `doc/errata-read-pdfa.md`), and ISO 32000-2's errata site for
   §7.3.10, §9.6.4's `d1`, §8.7.3 and §14.3; none changes a reading below.
2. **Probing, not only reading.** About ninety two-file probes (a file and its twin differing at one
   entry), each held to 2b, 2a or 4 by `examples/one.rs`, for every route a clause names: a key
   written as `null` and as a reference to no object, a stream's external-data keys, every action
   root and its `/Next` chain, a widget's inherited `/FT`, a degenerate `/Rect`, `Popup` and `Link`
   exemptions, the five page boundaries and an inherited `/MediaBox`, the implementation limits in
   a content stream, operators inside `BX`/`EX`, `q` nesting, the header, the `obj`/`endobj`/`xref`
   line endings, device colour through a shading, an annotation's appearance, a Type 3 glyph, a
   tiling cell, a form with its own resources under a page's `DefaultRGB`, the `CS` of a page, form
   and soft-mask group, an image's and a graphics state's intent, `RichMedia` and `Projection`
   under part 2, the optional content names and `Order`, language identifiers in an inline and a
   named property list, a font shown through an appearance, a `gs` `/Font` and mode 3, a page's
   metadata stream, and an embedded file reached through an annotation and a `GoToE` action.
3. **The converter's claim.** `pdf_transform::archive`'s census joins the validator's rows to
   `REMEDIES`, `WRITER_EMITS` and `REFUSED_BY_NAME`; `WRITER_EMITS`' eight rows were probed in the
   validator (each holds), and a converted output was put through veraPDF 1.30 (A691/A694: evidence
   about our reading, never the definition).

What holds, probed: the header's version digit per part and the binary comment; `/Encrypt`; the
`obj`/`endobj` and `xref` line endings; `/Length`; the implementation limits, in content streams
too; operators in `BX`/`EX`; every action root and its `/Next` chain, all eleven forbidden types,
the named actions; a widget's `/FT` inherited through `/Parent`; the appearance exemptions; the
page boundaries, inherited; device colour through a shading, an annotation appearance, a Type 3
`d0` glyph and a form whose own resources lack the page's `DefaultRGB` (TechNote 0010 A028); the
three transparency-group `CS` routes; optional content names, duplicates across `/D` and
`/Configs`, and `Order` over `/OCGs`; language identifiers in both property-list routes; font
embedding through an appearance and a `gs` `/Font`, mode 3 exempt.

## Findings

Five behaviour gaps. In order of how much a person relying on a verdict would lose:

- **2282-B02** — an embedded file whose file specification is written in place is never read.
  Part 2 section 6.8 binds the specification of every embedded file: the file itself conforms to
  part 1 or part 2, and the specification states `F` and `UF`. The population is the indirect
  objects plus the `/EmbeddedFiles` tree's direct leaves, so a file attachment annotation's direct
  `/FS` (or a `GoToE` action's `/F`) embedding a text file with no `UF` passes both rows. The
  converter then writes "the output was held to PDF/A-2b again and conforms"; veraPDF fails the
  same output on 6.8, test 5. A file sold as an archive carries an attachment no part admits.
- **2282-B03** — colour the base standard ignores is read as device colour. After `d1`, ISO
  32000-1 §9.6.5 (32000-2 §9.6.4) has "any use of such operators … ignored", and §8.7.3.3's
  uncoloured cell is painted in the colour its pattern is selected with. `rg` or `cs /DeviceRGB`
  in a `d1` glyph or an uncoloured cell, and an uncoloured cell that sets no colour (its "initial
  colour space"), each fail the `DeviceRGB` or `DeviceGray` row of section 6.2.4.3. The survey
  carries `State::uncoloured` for exactly this and gives it to the grammar only. A conforming file
  is failed, and the converter is sent to remedy a use the file does not make.
- **2282-B04** — a page's metadata stream that omits `/Type` is read by nothing. Section 6.6.2.1
  holds every metadata stream to the XMP rules; ISO 32000-1 Table 30 makes a page's `/Metadata`
  a metadata stream by where it stands. The population is streams stating `/Type /Metadata`, and
  no row reports the missing required `/Type` either, so a malformed packet with a deprecated
  `bytes` attribute passes. veraPDF is silent too (evidence only).
- **2282-B05** — `Order` is held to `/OCGs`, not to the groups in the file. Section 6.9 (part 4
  section 6.10) has an `Order` array reference every optional content group in the file. A group
  the page's content selects through `/OC` that `/OCGs` and `Order` both leave out is no group the
  row counts, and the base standard's rule that `/OCGs` lists every group is reported by no row.
- **2282-B01** — a forbidden key whose value names no object is failed as the key. §7.3.10: such
  a reference "shall be treated as a reference to the null object", and §7.3.7 makes a null entry
  the absent one. `table::states` is the validator's reading of both, and its comment says every
  rule that forbids a key asks through it; twelve ask `Dictionary::get(..).is_some()` or list
  `keys(..)` instead: a stream's `F`/`FFilter`/`FDecodeParms`, the catalog's and a page's `AA`,
  `Requirements`, `NeedsRendering`, `XFA`, `AlternatePresentations`, `PresSteps`, a
  configuration's `AS`, a widget's `A` and `AA`, an appearance dictionary's other keys, a
  permissions dictionary's keys, an image's `Intent` and a graphics state's `RI` (and, by reading,
  the `DocMDP` reference's digest keys). A conforming file is failed, and the converter is sent to
  remedy an entry that states nothing.

One documentation finding:

- **D01** — `order_lists_every_group` names the missing group with Rust's `Debug` form: the report
  prints `ObjectId { number: 7, generation: 0 }` where every other row prints `object 7 0`.

Not filed: part 2's NOTE that nothing follows `%%EOF` (a NOTE, and the row binds part 4 alone,
argued in `nothing_after_the_end`); the media-type row under part 4 section 6.9 (carried through
section 5.1 and §14.13.2, argued, and erratum 385 open); the recommendations (`Desc`, a Text
annotation's `NoZoom`/`NoRotate`, the information dictionary's consistency, section 6.7's level A
`should`s), each carried in `coverage` as stating no requirement; a key written as literal `null`,
which the parser already drops; `Encrypt` naming no object, which `pdf_syntax` refuses to open
(a reader's refusal, not a validator row).

## Ledger rows contradicted

`doc/conformance/ledger.toml` carries ISO 32000-2 and no row of it is contradicted: the 7.3.10,
8.6.8, 8.7.3.3, 8.11.2, 14.3.2 and 7.11.4 rows describe the readers, which read these right. What
is contradicted is the ISO 19005 coverage table (`pdf_archive::coverage`), whose `Carried::By`
names a row that does not read the route: part 2 6.8's embedded-file sentences
(`embedded-file-is-itself-pdfa`, `file-and-unicode-names`, B02), 6.2.4.3's three sentences (B03),
6.6.2.1's every-metadata-stream sentence (`xmp-packets-well-formed`,
`xmp-packet-header-attributes`, B04), 6.9's and part 4 6.10's `Order` sentence (B05), and every
forbidden-key sentence B01 lists. Each stays as it is until the fix round, which re-reads it.

## Suggested fix groups

1. **A forbidden key asks `states`** — B01 across `file_structure.rs` (`no_external_stream_data`,
   `permissions_dictionary_keys`, `document_signature_states_no_digest`), `interaction.rs`
   (`for_each_document_additional_actions`, `no_action_on_widget_or_field`,
   `no_additional_actions_dictionary`, `no_xfa_key`, `no_needs_rendering`,
   `appearance_dictionary_holds_only_normal`), `document_level.rs` (`AS`,
   `AlternatePresentations`, `PresSteps`, `Requirements`) and `graphics.rs`
   (`rendering_intent_entries_name_one_of_four`): `states`, and a `keys` that skips null values.
   The converter's matching remedies (`external.rs`, `actions.rs`) read the same reports.
2. **Every embedded file's specification** — B02: `for_each_embedded_file_specification` visits a
   direct specification wherever one stands (an annotation's `/FS`, an action's `/F`, an `/AF`
   element, a collection item), by walking the dictionaries the examination reaches rather than the
   objects; part 4's rows keep their own population. D01 is a line in `document_level.rs` beside it.
3. **Ignored colour** — B03 in `survey/colour_spaces.rs` and `survey/painting.rs`: `set_device_colour`,
   `set_space` and the initial-space record return without recording where `State::uncoloured`
   holds.
4. **Metadata streams and groups by position** — B04: `for_each_metadata_stream` and
   `metadata_streams` add every stream a `/Metadata` entry names, and a row (or
   `adheres-to-the-base-standard`) reports the missing `/Type`. B05: `order_lists_every_group`'s
   census is `/OCGs` together with every group the survey's `/OC` route reaches, and the base
   standard's `/OCGs`-lists-every-group rule is reported.
