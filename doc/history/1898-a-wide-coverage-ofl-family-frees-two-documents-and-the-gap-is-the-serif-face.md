# 1898 — A wide-coverage OFL family frees two documents, and the gap it closes is the serif face's

Slot 8 of batch 130, 2026-11-02, Opus 5.5, one session, a reading round: no code. ADR 2630 used; ADR
2631 and Q774 unused. Ledger: no row moved (the contract is A737's census, not a clause).

**Counted.** A737's item 2: against HEAD's two PDF/A census logs (no pack; a copy of the CJK pack),
every refusal and loss about a font the file does not embed (88 documents), each font's shown
characters classed by Unicode block off `mutool`'s extraction, and the twenty-six documents the
coverage sentences and the photograph gate name re-converted at HEAD under both profiles. A face
beyond the shipped ones frees two: `Deficient inflammasome …` (Greek, `Delta`, `mu` on `Times-Roman`;
refused under `as-shown`, 23 pages photographed under `as-if-printed`) and `issue5244` (Latin
Extended-A on an `Adobe-Identity` `TimesNewRoman,Bold`). Cyrillic, Arabic, Hebrew, Devanagari and
Thai free none: Liberation Sans already maps Cyrillic and Greek, and `ThuluthFeatures`' Arabic states
no `/ToUnicode`. The gap is Foxit Serif's and Fixed's standard Latin charset.

**Premise.** A737's and A569's words held; the brief's hypothesis ("a dozen documents, mostly Arabic
and Cyrillic") did not. The arms' reports carry no per-font report, so the rendering side was read
from the census and the converter's JSON, not the arms.

**Licence.** Noto's per-script `OFL.txt`, fetched into a fresh scratch directory: OFL 1.1, "The Noto
Project Authors", no Reserved Font Name, body the official text but for typography; `fsType` 0 in
every face. The row is drafted in `doc/third-party-data.md` as examined, not taken.

**Recommendation** (ADR 2630): the `latin-greek-cyrillic` package alone, as a second font pack beside
the CJK pack; the complex scripts wait for a route that does not draw unshaped text.

**Plant.** `Deficient inflammasome …` with Noto Serif supplied by `--font`, converted at HEAD: written at
2b and 4 under `as-shown` and passing veraPDF at both, written under `as-if-printed` with no page
photographed and its Greek extractable, page 1 looked at. It took a static instance, AGL glyph names
and a bare CFF program; the OpenType-CFF form was written with exit 0 and failed veraPDF 6.2.11.5
(`pdf_font::restate` restates `hmtx` and not the CFF charstring widths).

**Files.** `doc/adr/2630-…`, `doc/third-party-data.md` (one row), this record.

**Gates.** `cargo run -p conformance --bin cited` exit 0; `--bin quotations` exit 0; `cargo test -p
conformance --no-fail-fast` exit 101 on its first run, both reds mine (`adr_status`: ADR 2630 said
`proposed` to no question; `records`: this record's gates not yet written), then exit 101
on one red not mine (`conformance`'s citation test, three `§` in `crates/pdf-model/src/xmp.rs`, a
sibling's mid-edit; `adr_status` and `records` green). No
tier 1, no arms and no census: no code, no pixel and no conversion path changed.
