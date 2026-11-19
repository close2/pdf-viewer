# 3127 — A page reaching a font whose own name is not UTF-8 is photographed under `as-if-printed`

Session 2106, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R03 row
("left: a name the file's own program carries"). Status: accepted and built. In `pdf-transform`:
`archive/fenced.rs` (`bound_name_pages`, `is_a_font_part`, `with_font_parts`). Test:
`crates/pdf-transform/tests/archive/photographed.rs`
`a_page_reaching_an_embedded_font_named_in_no_utf8_is_photographed_under_as_if_printed`. Context:
ADR 2516 (the photograph under `as-if-printed`, A694 item 1), ADR 2846 (a font name restated
where the conversion embeds the program it names, and the rest of the row refused,
`doc/pdf-a-conversion-limits.md` section 4.7), ADR 2847 (a Type 3 font's pages found from the
object the validator names), ADR 2803 (an annotation's appearance keeps the refusal).
Clauses: ISO 19005-2 section 6.1.8; ISO 32000-2 §9.6.2.1 Table 109, §9.7.4.1 Table 117, §9.7.6.1
Table 119, §9.8.1 Table 120.

## Context

`file-structure/bound-names-are-valid-utf8` was a fenced row the photograph answered only where the
validator named a page: a name in a content stream. A font's name is reported at the object that
states it — the font, its `CIDFont`, its descriptor — and where the file embeds the program the
name is that program's own (Tables 117 and 120), so ADR 2846 restates it only where the conversion
embeds the program. `issue5010.pdf` shows an embedded `CIDFontType0C` font named in EUC-KR bytes
on its one page and stayed refused at both parts' 2b-level line.

## Decision

1. **The pages are the ones whose resources reach the font.** Where every place the row keeps is a
   font dictionary or a font descriptor (Table 109's and Table 120's `/Type`), or a page, the
   pages are each page whose resource walk reaches the object, the walk followed through a Type 0
   font's `/DescendantFonts` and every font's `/FontDescriptor`, which it did not follow before.
   Every such object has to be reached by some page; one an annotation's appearance reaches keeps
   the refusal, as ADR 2803 has it.
2. **The photograph takes the font out of the output** with the page's `/Resources`, as a print
   carries the glyphs and not their names — loss `text-as-image` and `page-as-image`, the page's
   text named in the report, as for every photographed page. `as-shown` keeps ADR 2394's gate.
3. **A colourant or a structure type name keeps its refusal**: a press finds an ink by its name,
   and a role map keys a structure type by it, from places no page's photograph takes.

## Consequences

`issue5010.pdf` converts at 2b under `as-if-printed`. The output is held to the target afterwards,
so a font a photograph left reachable from elsewhere makes the file refused rather than wrong.
