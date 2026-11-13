# 2858 — An appearance set in the stand-in is written under the loss `stand-in-face`

Session 2021, a single archive round.
Status: **accepted and built** in `pdf-model` (`appearance.rs`: `for_annotation_in`, `Written::stood_in`;
`variable_text.rs`: `LayoutFonts::without_the_stand_in_report`) and `pdf-transform`
(`archive/prepare/annotations_and_attachments.rs`: `prepare_appearances`; `archive/decision.rs`:
`Loss::StandInFace`, `answer_of`; `archive/decision/annotations_and_attachments.rs`: `CONDITIONAL`).
Context: ADRs 0112, 0816, 1105, 1425, 2504; R16 of `doc/pdfa-blocking-issues.md`; round 2016's record.
Clauses: ISO 32000-2 §12.7.4.3, §12.5.6.6, §12.5.5; ISO 19005-2 and ISO 19005-4 section 6.3.3.

## 1. What the census said

`poppler-395-0-fuzzed` holds two free text annotations with no appearance dictionary. Their `/DA`s
name `/Rufscript` and `/Helv`, and the file has no `/AcroForm`, so no `/DR` defines either.
§12.7.4.3 says the font value "shall match a resource name in the Font entry of the default resource
dictionary"; the file breaks that, and the standard states no recovery. `/Helv` is one of the
fourteen abbreviations `variable_text::STANDARD_ABBREVIATIONS` reads as §9.6.2.2's fonts, and it
owes nothing. `/Rufscript` is ADR 0112's stand-in: the window draws the value in it and reports the
missing definition. The conversion read that report as a partial rendering and refused the
appearance row (`APPEARANCE_INCOMPLETE`) under every profile at both parts. Under the four
profiles that state no recovered structure the file is also declined as a source (its `/Pages` does
not resolve), and under `no-data-loss` and `state-every-loss` its signature refuses as well.

## 2. Decision

1. **The stand-in is written where it is the whole of what is owed.** The construction's report is
   one sentence, and ADR 0112 names the stand-in ahead of a truncation, and a free text
   annotation's text ahead of its callout and border. So "whole" cannot be read from the sentence.
   `for_annotation_in` lays the annotation out a second time with the stand-in unreported
   (`LayoutFonts::without_the_stand_in_report`). `Written::stood_in` names the font only where that
   second layout owes nothing and draws the same stream. The stand-in still declines a value it
   cannot encode (ADR 0112's asymmetry), and that refusal is unchanged.
2. **The loss.** The face is this viewer's choice where the standard states none. Writing it into
   an archive fixes that choice for every later reader, and another reader may have set the text
   in another face. The word is `stand-in-face`. The appearance row is a `Conditional::PerDocument`
   of it: it is a statement where every constructed appearance draws what the clauses state, and
   this loss where one is set in the stand-in. The report names each annotation, its page and its
   font; the fonts row names the face it embeds and what that costs.
3. **The profiles.** `as-if-printed` takes it by `discard` at both appearance rows: the window
   draws and prints the stand-in, so the stream written is the window's drawing.
   `state-every-loss` takes it with `append-statement` first; this version composes no page at
   this site, so the report states the loss. `as-shown` does not take it. The stand-in is a font
   the file does not embed and whose name no face matches. Embedding a face for it is
   `machine-face` and `layout-from-this-viewer`, and `as-shown` stops at both. The first census
   with `as-shown` granting it wrote outputs that failed the fonts rows, which the census counts as
   the converter's own defect; refusing at the appearance row is the honest sentence. The other
   profiles grant nothing here and are asked.
4. **The near side of the authoring line** (`doc/scope-ratifications.md`). Nothing is invented.
   The text is the annotation's own `/Contents` and the layout is §12.7.4.3's construction, as for
   every constructed appearance (ADR 1105). The one thing this program chose is the face, and that
   is the loss.

## 3. Consequences

- `poppler-395-0-fuzzed` converts under `as-if-printed` at 2b and 4; veraPDF 1.30.3 passes both.
  `as-shown` asks for the loss at the appearance row, and under `state-every-loss` the signature
  still refuses it.
- The pixels (CPU, 72 dpi, `/OutputIntents` renamed, ADR 2691): the `/Helv` note is identical.
  "Phil Smith" in the stand-in differs in 468 pixels: its ink moves 0.34 px right and 0.22 px down,
  2 % lighter. The window draws `Rufscript` from a face of this machine, and the fonts row embeds
  the shipped Liberation Sans Regular in its place, which is the loss `machine-face` beside
  `layout-from-this-viewer` in the same report. The stream this decision writes is the window's.
- A stand-in beside a truncation (16384 codes, `variable_text::MAX_CODES`) is still refused as
  partial; the tests hold both cases and a profile that grants nothing.
