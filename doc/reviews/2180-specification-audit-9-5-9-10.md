# Specification audit of ISO 32000-2 §9.5 to §9.10 (font data structures) — round 2180

The project's tenth specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits 2128 to
2175 ([`2161-specification-audit-9-2-9-4.md`](2161-specification-audit-9-2-9-4.md)). Baseline:
main at `7368382a`. Evidence: [`2180-specification-audit-9-5-9-10/`](2180-specification-audit-9-5-9-10/)
— `findings.json` (2038's schema) and `expected-failures.json` (one failing test per behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing** but one doc slip (D01). A fix round takes the findings by
the groups at the end.

## Method

1. Every `shall` and `should` of §9.5 to §9.10 (Tables 108 to 127) was read in
   `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 7627 to 8712, extracted sentence by sentence
   (317 of them) and set against the code that reads the entry or performs the step. Most are
   constraints on the file (a writer's `shall`); those were read for what a reader does when a
   file breaks them, and only a silent wrong result was taken further.
2. 2038's findings on these clauses were read first and are not repeated: B15, B16, B27, B28,
   B50 (AGL sequences, `bfrange` destinations, source-code lengths, MacExpert's fill), T11 to
   T151 and D10 to D68.
3. The readers traced: `pdf-font`'s `truetype.rs` (§9.6.5.4), `glyph_names.rs` and
   `encoding.rs` (§9.6.5.1), `metrics.rs` (`/Widths`, `/W`, `/W2`, `/DW2`, `/MissingWidth`),
   `runs.rs` (§9.7.4.3's first-specification rule), `cmap.rs` (§9.7.5 and §9.7.6: codespace
   matching, §9.7.6.3's partial-match recovery, notdef mappings, `/WMode`, `usecmap`),
   `composite.rs` (`/CIDToGIDMap`, the CFF charset, `/UseCMap`, §9.10.2's third method),
   `program.rs` (Table 124's `/FontFile*`), `tounicode.rs` (§9.10.3) and `loading/` (§9.10.2's
   order, `.notdef` fallbacks, the vertical metrics); `pdf-model`'s `type3.rs` and
   `content/text/type3.rs` and `content/run/compat_type3_ops.rs` (§9.6.4 and Table 111).
4. The lens was the earlier audits' habits: a bound has two sides (Table 110's range); a rule
   keyed on a name applies wherever the name is read (an `/Encoding` §9.6.5.4 says is ignored,
   read by the glyph route and by text extraction); an inherited table is a third answer beside
   "stated" and "absent" (Table 118's `/UseCMap`).
5. Each candidate was probed through the public API — `CMap::parse`, `LoadedFont::load`,
   `Type3Font::read` — and each B finding has a failing `*_spec_audit` test whose control passes
   on main: `pdf-font/tests/fonts_9_5_9_10_spec_audit.rs` (its own binary) and
   `pdf-model/tests/type3_widths_range_spec_audit.rs`, a module of `text_suite`.

What holds, probed or traced and passing on main, is not listed one by one. Among it: a simple
font's `/Widths` bounded on both sides with `/MissingWidth` outside; Type 3 widths through
`/FontMatrix`'s `a` (the horizontal component of a rotated width); §9.6.4's resource search
ending at the page rather than the invoking form (EC3 Issue #128); `d1`'s colour and
transparency suppression; `/W` and `/W2` keeping a repeated CID's first statement; `/DW2`'s
default `[880 -1000]` and the position vector's horizontal half-width from `/W`; `/CIDToGIDMap`
as identity, stream (big-endian, short map to CID 0) and CFF charset; codespace matching byte by
byte, §9.7.6.3's partial-match recovery and notdef ranges giving one CID; Table 118's `/WMode`
checked against the file's; `vhea`/`vmtx` read by no drawing path; `PaintType` ignored; the
Symbolic bit read (both flags set reads as symbolic, as §9.8.2's `should` asks); the (3, 0)
subtable consulted before the (1, 0), per font; §9.10.2's three methods in order. Two readings
were left: whether `usecmap` inherits the referenced `CMap`'s `/WMode` is Adobe Technical
Note #5014's question and that note is not in `doc/`; and a (3, 0) subtable holding codes in two
of §9.6.5.4's four ranges at once is resolved by trying them in the clause's order.

## Findings

Three behaviour gaps. In order of how much a person would see:

- **2180-B01** — a referencing `CMap`'s `cidrange` does not override the referenced `CMap`'s
  `cidchar`: `Mapping::get` asks every single before any range, whichever file stated them, so a
  code the base states one at a time keeps the base's CID. The predefined `CMap`s a `/UseCMap`
  names state most of their codes that way, so a producer's override of a predefined map is the
  case that loses. Notdef mappings are absorbed the same way.
- **2180-B02** — a symbolic `TrueType` font's `/Encoding`, which §9.6.5.4 says is ignored, still
  selects glyphs for every code the (3, 0) and (1, 0) steps leave, above the reader-chosen tiers
  and unrecorded among them; the function's own documentation describes the font as reaching no
  rule. The same names feed §9.10.2's second method for a font whose selection used no name.
- **2180-B03** — a Type 3 font's `/LastChar` is never read, so a `/Widths` longer than the range
  advances codes past it where Table 110 gives them 0.

Four documentation findings: D01 (fixed here) `Type3Font::text`'s doc said only `/ToUnicode`
answers while the body applies the glyph-name method; D02 three comments in the Type 3 path carry
their own history; D03 `collection_table` quotes step b)'s "the font's CMap" and reads the
descendant's `/CIDSystemInfo`, unwritten as a choice; D04 a dead `UnsupportedEncoding` arm in
`truetype_code_table` whose comment describes a path `base_encoding` no longer takes.

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `9.7.5.3` (B01: its
note says `/UseCMap` is "followed"), `9.7.6.2` (B01), `9.6.5.4` (B02: "all five routes"),
`9.10.2` (B02's text half; D03), `9.6.4` (B03: "/Widths read through /FontMatrix").

## Suggested fix groups

1. **`pdf-font/src/cmap.rs`'s `CMap::absorb` and `Mapping::get`** — B01: the referencing file's
   mappings consulted before every inherited one, singles and ranges alike, for the character
   and the notdef mappings; `tounicode.rs`'s own-then-base split is the shape to share.
2. **`pdf-font/src/truetype.rs`'s `truetype_code_table`** with `glyph_names.rs` — B02 and D04:
   no name taken from an ignored `/Encoding`, every code past the clause's steps recorded as
   reader-chosen, and the `glyph_names` a symbolic font keeps for §9.10.2 decided in the same
   change; `pdf-archive`'s reader of `glyph_needed_a_reader_chosen_mapping` re-run after it.
3. **`pdf-model/src/type3.rs`** — B03 and D02: `/LastChar` read and applied as
   `metrics::stated_widths` applies it, and the three history comments restated as today's
   reasons.
4. **`pdf-font/src/composite.rs`'s `collection_table`** — D03: the descendant-for-`CMap` choice
   written down, or a predefined `CMap`'s own collection used where the descendant states none.
