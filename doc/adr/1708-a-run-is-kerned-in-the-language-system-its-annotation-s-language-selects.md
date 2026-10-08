# 1708 — A run is kerned in the language system its annotation's language selects

Session 1436. Status: **accepted** and **built**. Revisits ADR 1696 section 1's fourth choice ("the
language system is the default one"), which that ADR named as its cost; ADR 1696 is not edited.
Context: ISO 32000-2 §12.7.4.3, §14.9.2.1–§14.9.2.3, Table 29's and Table 166's `/Lang`; XFA 3.3
chapter 4 (*Localization and Canonicalization*, pages 152 and 153) and chapter 8 (*Layout for
Growable Objects*), held and cited by section; ISO/IEC 14496-22 (OpenType) *OpenType Layout common
table formats* (`Script` table, language system table, Example 3) and *Language system tags*
(OpenType 1.9.1), fetched from Microsoft's published text on 2026-10-08 and not held; RFC 5646
section 2.2.2; the Library of Congress's ISO 639-2 code list, fetched on 2026-10-08.
Code: `crates/pdf-font/src/pairs.rs` (`Language`, `registered`, `ScriptTable`, `kern_lookups_of`),
`crates/pdf-font/build.rs` (`language_table`, `two_letter_codes`), `crates/pdf-font/src/loading.rs`
(`pair_adjustments`), `crates/pdf-model/src/rich_text.rs` (`language`, `Chosen::language`),
`crates/pdf-model/src/rich_text/layout.rs` (`Request::language`, `Shaping::language`), one line of
`crates/pdf-model/src/appearance.rs` (`rich_laid_out`). Data: `data/opentype/language-tags.txt`,
`data/iso639/ISO-639-2_utf-8.txt`.

## 1. Which language names the language system

The brief asked whether a field's or the form's *locale* does. **XFA's `locale` does not**, for three
reasons the held texts state: it is a property of an XFA template's `draw`, `field` and `subform`
(chapter 4), which this tree does not read (§K.1's permission); what it governs is the localisation
of data — dates, numbers, currencies — and, in chapter 8, whether text flows right to left, never a
face's language system; and XFA rejects `xml:lang` outright, so a rich text string carries no
language of its own either. An interactive form field has no such entry.

**ISO 32000-2 names the language itself, in §14.9.2.** Its hierarchy starts at the catalog's `/Lang`,
"the default natural language for all text in the document" (§14.9.2.3), and Table 166 gives every
annotation a `/Lang` overriding it "for all text in the annotation". A widget is an annotation and a
field's value is the text it shows, so: **the widget's `/Lang`, else the catalog's**. A free text
annotation's text is its group's (§12.5.6.2), so the language is that annotation's. Table 226 has no
`/Lang` and Table 166's is not inheritable, so the field's parents are not walked. An empty
identifier is §14.9.2.2's unknown language and stops the walk; a tag that is not BCP 47 is unknown,
as `structure::document_language` already treats the catalog's. Unknown selects the default system.

## 2. Which language system a language selects

OpenType: a script table holds a default language system and zero or more for specific languages;
an application chooses one by its own criteria and uses one or the other, never both, so a
language system's lookups replace the default's (Example 3: they must list every lookup that applies).
The registry gives each language system tag the ISO 639 codes it corresponds to (`TRK ` for `tur`).
So a BCP 47 tag's language subtag, read through ISO 639-1's two-letter list to its three-letter code,
selects the registry's tags for that code; an extended-language subtag is tried first and its
macrolanguage after (RFC 5646 section 2.2.2 writes `ar-arz`, the registry lists `ARA ` for `ara`).
Within the run's script table, the first of those tags the table registers is used, else the default
system. Both lists are compiled in by `build.rs`; nothing is parsed at launch, and nothing is resolved
at open: an annotation's language is read where its rich text appearance is built.

**The table holds no reference, because a reference in a `static` costs every launch.** The first
build wrote `LANGUAGES` as `([u8; 3], &[[u8; 4]])` rows. Each slice is a pointer, and in a
position-independent executable each pointer is a relocation the dynamic loader applies before
`main`: 1 023 more `R_X86_64_RELATIVE` entries (14 573 → 15 596), and 12.3 thousand more
instructions in an open that reads none of them. `launch_path`'s counted open moved
`xfa_filled_imm1344e.pdf` 1 162.6 → 1 174.9 k (band 1148 .. 1172) and `opt_demo.pdf` 1 359.3 →
1 371.4 k (band 1340 .. 1368), measured as an A/B of HEAD `e833db88` exported twice, each with its own
target directory. So the tags are one flat `LANGUAGE_TAGS` array and `LANGUAGES` holds a code, a
start and a count, all plain numbers: 14 574 relocations, 1 162.7 and 1 359.0 k. `GPOS` is still chosen
over the `kern` table per program (ADR 1682 choice 1), now when any language system reaches a pair
lookup, so a program kerning only under Urdu kerns Urdu text and nothing else.

The costs, named. Script, region and variant subtags select nothing, so Chinese's five tags (`ZHH`,
`ZHP`, `ZHS`, `ZHT`, `ZHTM`) are tried in the registry's order rather than chosen by `Hans`/`Hant` or
region; the registry's eight tags with no ISO 639 code (the three phonetic ones answer to BCP 47
variant subtags) are never selected. A §7.9.2.2.2 escape sequence's language inside a value is
removed by the text string's decoding and not carried, and a structure element's `/Lang` over the
annotation is not read.

## 3. Evidence

Fixtures, because no corpus document states `kerning-mode` (ADR 1696's census). Four unit tests: the
mapping (`tr`, `TR-tr`, `ur-Arab-PK`, `deu`, `ar-arz`, the deprecated `DHV ` after `DIV `, seven
unknowns); every registry code reaching its own tag; a language system replacing the default; a
program kerning only in `URD `. `tests/rich_text.rs` writes `[(A) 97.65625 (VA) 97.65625 (V)] TJ` (the
Turkish −200/2048) for a widget's `/Lang (tr)` and for a catalog's `/Lang (tr-TR)`, and 146.48438 (the
default −300) for `de-DE` over `tr`, for `()` over `tr`, and for none. Three planted defects — the
language system never matched, the widget's `/Lang` not read, the two-letter list skipped — each
failed the tests. Both pages were rendered and looked at.
