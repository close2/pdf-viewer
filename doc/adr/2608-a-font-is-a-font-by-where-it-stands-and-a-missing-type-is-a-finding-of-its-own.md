# 2608 — A font is a font by where it stands, and a missing `/Type` is a finding of its own

Status: implemented, in `pdf-archive` (`table::fonts`, the font population and the row
`fonts/font-dictionary-states-type-font`; `examination`, the population's one walk; `coverage`,
section 6.2.11.2 and section 6.2.10.2) and `pdf-transform` (`archive::decision`, the row's
`Answer::Mechanical`; `archive::sites::font_types` and `typed_fonts`, where the entry is written;
`archive::rewrite::values::state_font_type`, the writing; `archive::fonts::gains`, a direct font
written out as an object of its own).
Session 1887, the first robustness slot of batch 129.
Context: ISO 32000-2 §7.8.3's Table 34, §9.6.2.1's Table 109, §9.6.4's Table 110, §9.7.4.1's
Table 115 and §9.7.6.1's Table 119; ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2;
ADR 2413, the population's direct half.

## The gap

`is_a_font` recognised a font dictionary by `/Type /Font` alone, so a font the file uses without
that entry escaped every font row. pdf.js's `issue7544.pdf` is the shape: its Type 0 font states
`/Type /Font`, and its one descendant, the `CIDFontType2` the text is drawn with, states no `/Type`.
veraPDF fails it on 6.2.11.2-1 at 2b; the checker passed it. The premise handed to this slot named a
font a page's `/Font` resource names; the file's resource names the typed Type 0 font, and the font
without `/Type` is the descendant.

## Decision

1. **A font is a font by its `/Type` or by its position.** Table 34 makes a resource dictionary's
   `/Font` a map from resource names to font dictionaries, and Table 119 makes `/DescendantFonts`
   the array naming the `CIDFont`; a dictionary standing in either place is the font the file uses
   whatever it states. Arlington's model has no other dictionary with a `/Font` dictionary or a
   `/DescendantFonts` entry; a graphics state's `/Font` is an array and names no holder here.
2. **One walk finds both halves.** The walk ADR 2413 added for direct fonts now also reads each
   holder: an element that is a reference puts its object number in the position set, a direct
   dictionary that does not state `/Type /Font` joins the direct half, and a holder that is a
   reference is read once after the walk. Every font row's `for_each_font` visits the union, so the
   descendant above is judged by `cid-to-gid-map-present` and the rest as the `CIDFont` it is.
3. **The missing entry is a row of its own**, `fonts/font-dictionary-states-type-font`, under both
   parts' section that binds every font to the base standard's font clauses. A dictionary whose
   `/Type` is another name is the same finding with the name said. Other dictionaries without their
   `/Type` (a descriptor, an encoding) are not this row.
4. **The converter writes the entry**, `Rewrite::FontTypeStated`, the shape of
   `Rewrite::FileHeader` (ADR 2558): the key is required and admits one value, so the name written
   is decided by the four tables and the dictionary's position, never by the converter, and it
   replaces a name that was another. A font that is an object of its own is given it in its
   object; a font written directly inside another object, which the row reports at no object
   (ADR 2413), is given it inside its holder, which `pdf_archive::direct_fonts_stating_no_font_type`
   names, and the requirement is refused by name unless the holders hold exactly as many such
   fonts as the row reported. A direct font the substitution writes out as an object of its own
   (ADR 2162) is left untouched where it stands, because the substitution finds it there by its
   value, and its new object states the entry instead; written first, the entry hid the font from
   the substitution and the output failed `font-programs-embedded` (the plant below).

## Bounds

No new budget: the walk is linear in what the parser produced, and each holder object is read once
(a set of holders, not a list). `getrusage`'s high-water for `quorra-transform archive --to 2b`, a
page whose `/Font` resource holds 50 000 fonts, HEAD beside this change: written directly and
without `/Type`, 239 056 kB (converted) and 268 052 kB (refused by the row), where 50 000 such fonts
stating `/Type /Font`, which HEAD already walked, take 271 892 kB and 271 464 kB; as 50 000 objects
without `/Type`, 173 164 kB and 120 680 kB. The untyped population costs what the typed one
already did.

## What it moves

The row alone, refused, moved seven documents from "converted" to refused under `as-if-printed`
in the census without the CJK pack (pdf.js `freetexts.pdf` and `issue16633.pdf`; `T02-04-01_003`,
`T02-04-01_004`, `AcroFormsBasicFields.pdf`, `transitions_test.pdf`, `page_tree_multiple_levels.pdf`),
and `issue7544.pdf` named it beside its own refusal. veraPDF fails HEAD's PDF/A-2b output of each
of the seven on 6.2.11.2-1 and on nothing else; a control output (`tracemonkey.pdf`) passes. With
the entry written, the census is HEAD's line for line: the seven convert again, now with the key
veraPDF asked for, and veraPDF passes each of the fourteen outputs at 2b and at 4, where HEAD's
PDF/A-2b output of `freetexts.pdf`, run first as the planted failure, fails.

## Plants

The position walk switched off: the fixture font without `/Type` went unjudged (left 0, right 1).
The substitution's skip switched off: a direct untyped font given a face did not convert.
