# 1696 — A run is kerned by the script table its characters select, and a contextual lookup is not a pair

Session 1430. Status: **accepted** and **built**. Revisits ADR 1682 section 3's choices 2 and 3,
which that ADR named as the two to revisit; ADR 1682 is not edited. Context: ISO 32000-2 §12.7.4.3;
XFA 3.3 chapter 27 (*Kerning*, pages 1203 and 1204) and the version 2.8 change list (*Pair kerning
support*), held and cited by section; ISO/IEC 14496-22 (OpenType) *OpenType Layout common table
formats* (`ScriptList` table, `Script` table) and *Script tags* (OpenType 1.9.1), fetched from
Microsoft's published text on 2026-10-08 and not held; the Unicode Character Database 18.0.0's
`Scripts.txt` and `PropertyValueAliases.txt`, held.
Code: `crates/pdf-font/src/pairs.rs` (`Scripted`, `Script`, `scripts`, `tags`,
`kern_feature_lookups`), `crates/pdf-font/build.rs` (`script_table`),
`crates/pdf-font/src/loading.rs` (`pair_adjustments`), `crates/pdf-model/src/rich_text/layout.rs`
(`Shaping::script`, `kern`). Data: `data/unicode/Scripts.txt`, `data/unicode/PropertyValueAliases.txt`,
`data/opentype/script-tags.txt`.

## 1. Which `kern` lookups a run takes

ADR 1682 took every `kern` feature record of `GPOS` together, so a program registering a different
`kern` lookup for one pair under two scripts applied both. OpenType does not: a run of text is laid
out under one script table — the one registered for the script of the text, `DFLT` where the
program registers none for it or the text has no specific script (the `ScriptList` table) — and
under one language system of it, whose feature indices, the required feature among them, are the
features applied. So:

- **A character's script** is its Unicode `Script` property, compiled in from `Scripts.txt` by
  `build.rs` as ISO 15924 codes (`PropertyValueAliases.txt`'s `sc` aliases), nothing parsed at
  launch. `Common` and `Inherited` take the script of the nearest specific character before them,
  or after them where none precedes — the processing the `ScriptList` note describes for
  script-neutral characters — resolved across the whole rich text string, once, the first time a
  run asks for pair kerning. A string of neutral characters alone has no script.
- **A script's tag** is the registry's: the code in lower case, but for the five scripts the
  registry spells otherwise (`kana` for Hiragana, and `lao `, `nko `, `vai `, `yi  `), and the ten
  with a "v.2" tag, which is tried before the first. `data/opentype/script-tags.txt` holds the
  registry's list, and `every_script_reaches_a_registered_tag` reads every code against it.
- **The script table** is the first of those tags the program registers, else `DFLT`, else none. A
  table registered for the script that reaches no `kern` lookup gives the run none; it does not
  borrow `DFLT`'s, because OpenType uses `DFLT` only where no table for the script exists.
- **The language system is the default one.** A field's text names no language system, and
  OpenType has a run use the default one in the absence of language-specific information.
- **Two glyphs of different scripts are not a pair**, in `GPOS` and in the `kern` table alike: a
  layout is applied one script run at a time. Two scripts sharing one tag (`Hira`, `Kana`) are one run.
- **`GPOS` or the `kern` table** is still decided per program (ADR 1682 choice 1), now by whether
  any script table's default language system reaches a pair lookup through `kern`; a `kern` feature
  record no such system names is not applied, and the program falls to its `kern` table.

The cost, named: a language-specific system (Turkish `latn`, Urdu `arab`) is never chosen, and a
program that kerns only there is not kerned by it.

## 2. A contextual lookup stays out, and why

The change list defines pair kerning as kerning based on the two adjacent glyphs alone, and chapter
27 says more kinds of kerning may be added later. A `ContextPos` or `ChainContextPos` lookup under
`kern` positions a pair by the glyphs before or after it: it is another kind of kerning. It is not
applied, and nothing is said, because the property is fully carried out without it. A program whose
`kern` feature reaches only contextual lookups states no pair kerning and is said as such. The cost,
named: a contextual subtable whose context is just the two glyphs is pair kerning in a contextual
format, and it is left out with the rest.

## 3. Evidence

The fixtures are Liberation Sans with its `GPOS` and `kern` removed and one known `GPOS` written in
(trap 8). Seven unit tests: neutral resolution; the registry; a run takes its own script's lookups
(Latin −300, Cyrillic and digits fall to `DFLT`'s −100, where ADR 1682's union gave −400); a script's
own table does not borrow `DFLT`'s; no pair across a change of script; `dev2` before `deva`; and a
`ChainContextPos` reaching a pair lookup not applied, beside a direct pair that is. Four planted
defects each failed the tests: `DFLT` taken first, every lookup applied, a wrong Lao tag, and no
split at a script change. `tests/rich_text.rs` writes `[(A) 146.48438 (VA) 146.48438 (V)] TJ` from
`latn`, where scripts left out wrote 48.828125 and failed. The page was looked at.

**The corpus.** A census decoding every stream and every `/RC`, `/RV` and `/DS` string of 1489
unique documents (the pdf.js set, the four submodule corpora, `doc/corpora-own` and the residue
corpora) found `kerning-mode` or `kerningMode` in none. A planted document carrying both was found.
So the fixtures are the only evidence of this property, as they were before.

## 4. What stays

§9.6.2.2's fourteen, until `doc/questions/Q308` is answered. The three rows stay `partial` on that
alone.
