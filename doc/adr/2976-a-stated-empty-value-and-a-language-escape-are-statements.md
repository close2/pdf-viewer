# 2976 — An empty `/Lang`, an empty `/ActualText` and a language escape are statements

Session 2075. Status: **implemented.** Context: review 2038 (findings B01, B02, B12, B30, B31, B34, T03), ADR 2812
§2 (whose last sentence this supersedes). Clauses: ISO 32000-2 §7.9.2.2.2, §14.9.2.2, §14.9.2.3, §14.9.3, §14.9.4,
§14.9.5.

**Empty `/Lang`.** §14.9.2.2: an identifier "shall either be the empty text string, to indicate that the language
is unknown, or a Language-Tag". The reader dropped the empty string as though the entry were absent, so an unknown
span inherited the catalog's or an outer span's language. It is now kept as `Some("")` on `Described::language`
and by `structure::language`, it ends §14.9.2.3's inheritance walk as any stated tag does, and it reaches a
text-to-speech run or an accessibility node as no language. The catalog's own `/Lang` is unchanged (Table 29's
erratum makes an absent or invalid one unknown already).

**Empty `/ActualText`.** §14.9.4 makes the entry replacement *character content*; the empty string replaces the
enclosed content with nothing, which is how a producer drops a discretionary hyphen. It is now kept, on a `Span`, an
`Artifact` property list and a structure element alike. An empty `/Alt` or `/E` still states no phrase: neither
clause describes speaking nothing in place of an item, and B34 does not adjudicate them.

**A structure element's `/ActualText` covers what the element encloses, once.** §14.9.4 substitutes for "the
content enclosed by the structure element". The reader applied it per marked-content sequence, and only where the
sequence's own element stated it. The parent tree now names the element by reference (`ParentTree::element_id`),
one walk up `/P` (`structure::enclosing`) finds the nearest `/Lang` and the outermost `/ActualText`, and the page
records each element whose replacement it has placed: the first content item carries the text and later ones are
replaced by nothing. Content items that are not adjacent get the replacement at the first of them.

**Escapes in `/Alt` and `/E`.** §7.9.2.2.2's escape overrides "the prevailing Lang entry" in any text string.
ADR 2812 §2 left `/Alt` and `/E` with the escape removed and the section's language spoken, and said a phrase
changing language mid-string was not split. That sentence is superseded: `Described::phrase_languages` carries the
escapes of the spoken phrase, `speech` splits the phrase at each, and an accessibility node whose `/Alt` or `/E`
opens with an escape is in that language. An annotation's `/Contents` spoken in its place carries its escapes too.
