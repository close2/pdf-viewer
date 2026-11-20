# 3356 — A Type 3 glyph's own text object moves no cursor of the page's readback

Session 2163. Status: accepted. Context: audit 2161's finding B05; ADR 1515.

## The clauses

§9.4.1's NOTE: "text can be shown using a Type 3 font whose glyph descriptions include any graphics
objects, including another text object". §9.6.4: a glyph description "contains the operators that paint
the glyph". §9.4.4 advances the page's Tm by the glyph's displacement alone.

## Decision

`read_back` already read nothing from inside a glyph description (`glyph_depth > 0`); the gap measure
`separate_text` and the cursor it measures from did not, so a description that showed text moved the
cursor into its own space and the page's next show read a word break the page never placed. Both are now
gated the same way: `separate_text` returns inside a description, and the cursor moves only through
`move_text_cursor`, which keeps it at depth zero. The description's text is the glyph's implementation,
and the page's own advance is the only position its readback measures from.
