# Specification audit of ISO 32000-2 Annexes D, E, F and I (character sets and encodings, extending PDF, linearized PDF, versions and compatibility) — round 2260

The project's twenty-first specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2253 ([`2253-specification-audit-14-1-14-5-14-10-14-13.md`](2253-specification-audit-14-1-14-5-14-10-14-13.md)
is the one before). Baseline: main at `698528a7b`. Evidence:
[`2260-specification-audit-annexes-d-e-f-i/`](2260-specification-audit-annexes-d-e-f-i/) —
`findings.json` (2038's schema) and `expected-failures.json` (the failing test of each behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. The `shall`s and `should`s of Annexes D, E, F and I (`doc/md/ISO_32000-2_sponsored_EC3.md`
   lines 19068 to 20404 and 20895 to 20928) were read with the ledger's thirty-seven annex rows.
   Errata: `doc/errata/site/docs/32000-2-2020/clauseAnnexD.md`, `clauseAnnexE.md`,
   `clauseAnnexF.md` and `doc/errata-read.md` — #562, #285 and #461 on Table D.3 (0x04/0x05
   aliases, 0x18 breve and 0x19 caron, 0x16 as U+0016, 0x8A minus), #229 and #340 on E.2, #699 and
   #687 on F.3.1, **#331** on F.3.2 (linearization floor 1.2), #153 and #389 on Table F.1, #403 on
   F.3.3 and F.3.6.
2. Annex D's six tables were diffed mechanically against `pdf-font/src/encoding.rs` (WinAnsi,
   MacRoman, MacExpert, Symbol, ZapfDingbats; StandardEncoding through `read-fonts`' CFF table) and
   Table D.3 (errata applied) against `pdf-syntax/src/text_string.rs`: **no code disagrees**, the
   extra WinAnsi codes are Table D.2's notes 3, 5 and 6, and every `U` code is undefined. A probe
   found every one of D.2's 229 names reaching a glyph in all twelve Times, Helvetica and Courier
   faces (D.1's support `shall`).
3. Annex E: a census of every name literal the writers construct (`pdf-transform`, `pdf-syntax`'s
   writers, `pdf-model`, `pdf-archive`, `viewer-core`, `viewer-host`, `pdf-signature`) against
   the Arlington model found no developer-extension key or value.
4. Annex F: `linearize.rs` was read against F.3 and F.4 (the parameter dictionary, `/H`, the
   first-page section and trailer, part 4's entries including `/Encrypt`, hint-stream numbering,
   the F.3.2 floor of #331), and **every reader of Table F.1** was listed — `linearize::state`,
   `pdf-archive`'s `pages_held`, the converter's `recovered`.
5. Annex I: `Document::version`, the viewer's newer-version note, and the version each writer
   chooses (`merge`, `split`, `optimize`, `redact`, `update`'s insertion, the serializer's raises,
   the linearizer, the archive converter).
6. Habits applied: *every reader of a dictionary is a route to it* (B02); *a rule about how a token
   is written binds every writer that formats one by hand* (B03); *an absent entry is a third
   answer* — a name a table does not list is treated as absent, and the annex puts
   `StandardEncoding` among those names (B01).

What holds, probed by reading: Table D.3 as amended; the UTF-16BE/PDFDocEncoding choice of
`encode_text_string`; Annex E's classes in everything written; the linearizer's parts 1 to 11, its
first-page trailer (`/Size` combined, `/Prev`, `/Root`, `/ID`, `/Encrypt`), its `/O` from
§12.6.4.2's `/OpenAction` through `pdf_model::destination`, part 4's §F.3.5 entries, and the 1.2
floor; Annex I's "should never be changed to an older version" in `merge` (the highest source
version), `update`'s insertion (a catalog `/Version`), `split`, `optimize` and `redact` (the
source's), the encryption and cross-reference-stream raises, and the viewer's note for a version
past 2.0.

## Findings

Three behaviour gaps. In order of how much a person would lose:

- **2260-B02** — a stale `/N` decides a conversion. The converter's rebuild and `pdf-archive`'s
  page check read `/N` from any `/Linearized` dictionary, never asking whether `/L` is still the
  file's length; Table F.1 makes such a file ordinary and its linearization information ignored.
  An incrementally updated file that deleted pages is refused for pages it no longer states, or
  gets an appended page stating a count it does not make. `linearize::state` holds the rule and
  neither caller uses it. The existing test at `tests/archive/recovered.rs:786` asserts the
  refusal over a mismatched `/L` and changes with the fix.
- **2260-B01** — `StandardEncoding` is a predefined encoding here, and Table D.1 says it "shall
  not" be. A nonsymbolic font naming it is read through Table D.2's STD column instead of its
  program's built-in encoding; an embedded Type 1 program whose built-in encoding has `A` at 66
  draws nothing there. The symbolic standard-14 fonts are unaffected.
- **2260-B03** — `merge::text_string` (merge's and split's page labels) writes ASCII unprefixed,
  that is in PDFDocEncoding, where 0x18–0x1F are accents and 0x00–0x08 and 0x7F undefined. A
  label with U+001F reads back as a tilde.

Four documentation findings: **D01**, the D.1 row counts two of the annex's four `shall`s and
calls `MacExpertEncoding` refused; **D02**, `PERMITTED_ENCODING_NAMES` says Annex D defines the
name it forbids; **D03**, the E.2 row names one writer-chosen name where there are eight; **D04**,
`version_for` lowers a `%PDF-1.8` or `1.9` source to 1.7 with no record, which Annex I's *should*
and ADR 2692's record for a 3.x source both leave unanswered.

Not filed, for the fix round to test first (by reading): the converter's output intent `/Info`
(`archive/prepare/colour_and_graphics.rs:659`) is a text string written as the ICC `desc` tag's
UTF-8 bytes with no prefix; a profile whose description is not ASCII is read back as
PDFDocEncoding mojibake. B03's group takes it.

## Ledger rows contradicted

Each stays as it is until the fix round, which re-reads it: `D.1` (B01, D01), `F` and `F.3.3`
(B02 — the F row says the reader's half is `linearize::state`, and two readers are not), `D.3`
and `7.9.2.2` on the write side (B03), `E.2` (D03, the conclusion holds), `I.2` (D04). All are
`implemented` except E.2, `writer-side`.

## Suggested fix groups

1. **One reading of Table F.1** — B02: both `/N` readers through `linearize::state`; the
   `recovered.rs:786` fixture given a matching `/L`.
2. **Text strings written by hand** — B03 and the output intent `/Info`: `encode_text_string`
   in both; a grep for `Object::String(` over `as_bytes`/`into_bytes` in the writers first.
3. **The predefined encodings** — B01, D01, D02: `/StandardEncoding` treated as Table 109's
   other unlisted names, the docs and the D.1 row rewritten; a corpus census of fonts naming it
   over a non-standard built-in encoding before the change (a question if one relies on it).
4. **Notes** — D03, D04.
