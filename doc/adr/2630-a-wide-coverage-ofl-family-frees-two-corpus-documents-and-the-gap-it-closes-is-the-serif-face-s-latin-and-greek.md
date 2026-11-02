# 2630 — A wide-coverage OFL family frees two corpus documents, and the gap it closes is the serif face's Latin and Greek, not a script beyond CJK

Status: accepted — the order of the work A737 already owes, not the owner's choice of whether. Session 1898, the reading slot of batch one hundred and thirty. Context: owner
answers A47, A569 and A737 (its `Owes:` item 2: a census of what a wide-coverage OFL family for the
scripts beyond CJK would free, and its licence read, before any pack is built); ADR 2460 (the CJK
pack and how a census states it); ADR 2394 and A694 item 1 (the photographed page and its wider
gate); ADR 2378 (an `Adobe-Identity` font re-keyed from its `/ToUnicode`); `doc/cjk-pack.md`.

## What was counted, and how

Against HEAD's two PDF/A census logs (no pack, and a copy of the CJK pack), every refusal and every
loss whose sentence concerns a font the file does not embed — "no face this program ships",
"drawn by neither", `machine-face`, `layout-from-this-viewer`, a composite font with no program,
a standard-14 name, an `Adobe-Identity` font, and the photograph gate's own refusals — was listed by
document, profile and part (88 documents). For each, the characters every unembedded font shows
were read off `mutool`'s text extraction (evidence about which script a font sets, nothing more)
and classed by Unicode block; the twenty-six documents the font sentences or the photograph gate
name were converted again at HEAD under both shipped profiles with the JSON report. The census is
a population, not a definition: the requirement stands whatever it counts (A569's reading, the
two denominators of `CLAUDE.md`).

## The table

| script | documents | the line each would move | the family's file, per script |
|---|---|---|---|
| Greek (and `Delta`, `mu`, `minus` beside it), on `Times-Roman` | 1: `Deficient inflammasome activation …` (corpora, format-corpus) | `as-shown` 2b and 4: refused "no face this program ships has a glyph for every code it shows" → written; `as-if-printed` 2b and 4: pages 1–23 photographed → set as text | Noto Serif: 1 887 192 B variable; 513 124 B one static Regular instance (`glyf`), 838 860 B as an unsubroutinised CFF |
| Latin Extended-A (ą ć Ċ ē Ĩ Ī ń ż), on `TimesNewRoman,Bold`, `Adobe-Identity` with `/ToUnicode` | 1: `issue5244` (pdf.js) | `as-shown` 2b and 4: refused "drawn by neither the face this program ships … nor any installed CJK pack program" → written; `as-if-printed`: page 1 photographed → set as text, once ADR 2378's face order takes a pack face after the shipped one and the CJK pack (not planted: `--font` does not reach this route, whose supplied program is keyed by glyph identifier) | Noto Serif Bold, the same package |
| Cyrillic | 0 (379049 and issue2840 show it, and already convert: Liberation Sans maps 100 Cyrillic code points) | none | Noto Sans 2 049 096 B variable, the same package |
| Arabic | 0 (`ThuluthFeatures` is Arabic, but its `Identity-H` font states no `/ToUnicode`, so no face can be given characters nobody named — ADR 2394's photograph stays right) | none | Noto Sans Arabic 844 676 B, Noto Naskh Arabic 307 592 B |
| Hebrew, Devanagari, Thai | 0 | none | 112 640 B, 641 944 B, 218 652 B |
| CJK | the pack's (thirteen documents differ between the two logs, all `machine-face`) | ADR 2460's | — |

Nothing else in the population moves. The eight `layout-from-this-viewer` documents show Latin only;
the twenty documents the photograph gate refuses ("content this reader does not draw whole") show
Latin and symbols only on their unembedded fonts, so their cause is not a face; the four
standard-14 sentences are width or machine-face questions; the three "no program was named"
documents are CJK. **The hypothesis — a dozen documents, mostly Arabic and Cyrillic — did not
hold**: two documents, Greek and Latin Extended-A, and both because the serif and fixed faces this
program ships (Foxit Serif 230 glyphs, Foxit Fixed 244, no Greek, no Cyrillic, no Latin
Extended-A) are narrower than the sans face (Liberation Sans: 330 Latin, 73 Greek, 100 Cyrillic
code points).

## The licence, read off copies

Each per-script Noto repository (`notofonts/arabic`, `hebrew`, `devanagari`, `thai`,
`latin-greek-cyrillic`) carries `OFL.txt`: SIL Open Font License 1.1, one copyright line ("The
Noto Project Authors"), **no Reserved Font Name**. The body matches the official OFL 1.1 text but
for typography (straight quotes, `--`, numbering as `1)`). Its terms, by section and condition:
the PERMISSION & CONDITIONS section allows use, study, copying, merging, embedding, modification,
redistribution and sale; condition 1 forbids selling the fonts by themselves (bundling is
allowed); condition 2 requires each copy, original or modified, to carry the copyright notice and
the licence; condition 3 reserves a Reserved Font Name — none here, so a rebuilt program may keep
"Noto"; condition 4 forbids using the authors' names to promote a modified version; condition 5
keeps the fonts under the OFL alone, and states that documents made with them are not thereby
under it. Every font fetched states `fsType` 0 (installable embedding), which is what ISO 19005-2
section 6.2.11.4.1's lawful embedding asks the converter to be able to state. The obligations are
the CJK pack's (`doc/cjk-pack.md`, A550's pattern): `LICENSE.txt` in the pack, the notice in
`/NOTICE`, and the licence in each subset's `/Metadata`.

## What the plant showed, and what a pack's builder therefore owes

The Greek document, converted at HEAD with Noto Serif supplied by hand (`--font Times-Roman=…`),
is written under `as-shown` at 2b and at 4 and passes veraPDF at both, and its first page draws the
β in the serif face at the file's widths; under `as-if-printed` at 2b it is written with no page
photographed and its Greek extractable. It took four builds of the face to get there, and each is
a requirement on a pack:

1. **google/fonts holds only variable fonts and the notofonts repositories hold no binaries on
   their default branch**; a pack instances one static face per weight (fontTools' `instancer`)
   from a pinned commit, as `tools/cjk-pack.py` pins Source Han's.
2. **A `glyf` face is refused for a `Type1` dictionary when supplied** (Table 124); a shipped face
   is re-filed as TrueType the way Liberation's is (`fonts/described.rs`), so the pack's faces
   either take that shipped route or are written as bare CFF.
3. **Glyph names must be the Adobe Glyph List's**: a simple font reaches its glyphs by name, and
   Noto's `uni03BC` and `uni0394` are not `mu` and `Delta`.
4. **An OpenType-CFF face fails ISO 19005-2 section 6.2.11.5 at veraPDF** (375 checks) while this
   checker passes it: `pdf_font::restate` restates an sfnt's `hmtx` and leaves the `CFF ` table's
   charstring widths, which is what veraPDF reads. The same face as a bare CFF passes. Found, not
   fixed (the record names it).

## The recommendation, and the decision

**One package only, now: Noto's `latin-greek-cyrillic` — Noto Serif and Noto Sans Mono in the four
styles the Foxit faces ship (Noto Sans Mono's coverage not yet read) — as a second font pack beside the CJK pack** (A569's shape, A737 item 1), the
fallback for a code the Foxit serif or fixed face cannot draw. It frees the two documents above and
closes, for every document, the class they belong to: Latin Extended, Greek and Cyrillic in a serif
or monospaced font. **Arabic, Hebrew, Devanagari and Thai stay owed and are not worth a pack yet**,
for a reason beyond the census's zero: every route that would use them addresses glyphs one
character or one code at a time, and Devanagari's and Thai's glyphs are chosen by the face's
`GSUB` from character sequences, so a face given the characters alone would draw unshaped text —
a layout this program would be inventing rather than the producer's. Arabic is the exception in
part: Noto's Arabic faces map 772 presentation-form code points, so text a producer encoded
already shaped could be served; a census of documents showing presentation forms is the trigger
that reopens it.

## Consequences

- No code and no dependency: the row in `doc/third-party-data.md` is drafted as examined, not taken.
- A round building the pack takes the four builder requirements above, and the restatement defect
  of item 4 before it.
