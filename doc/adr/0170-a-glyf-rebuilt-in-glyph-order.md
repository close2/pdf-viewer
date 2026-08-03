# ADR 0170 — A `glyf` rebuilt in glyph order, because an entry is self-describing

Status: accepted, 2026-08-03. Session 223, one session after the diagnosis.
`doc/todo/13-a-loca-table-that-goes-backwards.md`, now deleted.

## The defect

`issue11131_reduced.pdf` draws *Operating Account Consolidated Statement* in an embedded
`CIDFontType2` subset. This tree drew about half the glyphs —
`p  r ti g   ou t o soli t  St t     t` — and **reported nothing**, because `FontError` is per
font and this font loaded and produced glyphs. The three reference renderers that draw the whole
sentence are the three that share `libfreetype`; the two that draw half are the two that read a
font with `skrifa`.

A glyph's data runs from `loca[i]` to `loca[i + 1]`, so the offsets ascend by construction. This
font's are `16776 16776 16776 16776 10674 2188 2590 1886 …`: **36 of its 71 glyphs state a
negative length**, `read-fonts` refuses each of them, and FreeType instead takes the entry's
extent from the entry.

## Why the repair is a derivation

**A `glyf` entry is self-describing.** `numberOfContours` decides simple or composite; a simple
glyph's extent follows from its contour count, its instruction length and its run-length-encoded
flag stream; a composite's from its component loop up to the entry with `MORE_COMPONENTS` clear.
So the file states each glyph's extent twice — once in `loca`, once in the entry — and only one
of the two readings is self-consistent. That is the same shape as `repaired_loca_format`'s
byte-swapped `indexToLocFormat` one table over, and as the twenty-seventh session's LZW finding:
**a file that states one fact twice can check itself**, and no other implementation's behaviour is
involved.

What is *not* available is that function's two-byte edit. `loca[i + 1]` is also glyph `i + 1`'s
start and is right as such, so no single entry can be rewritten without breaking its neighbour:
the glyphs sit in `glyf` in an order the offsets do not follow. `repaired_loca_order` therefore
puts them in one — a new `glyf` with each glyph's own bytes in glyph order, and a new monotonic
`loca` beside it. Glyph ids do not move, so a composite's references stay valid, and every other
table is copied through.

## Two decisions worth naming

**Append rather than rebuild the file.** The new tables go on the end and the directory entries
are repointed, which leaves every other table where it was — so `head`'s offset, read *before* the
repair, is still right afterwards, which is what lets the caller patch `indexToLocFormat` to the
long form in two bytes. The cost is the size of the table being replaced, left in the file
unreferenced. `checkSumAdjustment` is not recomputed, as it is not by any producer that edits a
font in place.

**The common path is one pass and nothing else.** `repaired_loca_order` reads `loca` once, returns
`None` the moment the offsets ascend, and allocates nothing. Every well-formed font leaves there.

## What the corpus says, measured

Every `/FontFile2` in all 974 documents, extracted and read: **623 embedded TrueType programs
carry a `loca`, and six of them are not monotonic** — `bug1650302_reduced.pdf` (38 descending
pairs of 5394 glyphs), `bug868745.pdf` (4 of 77), `issue11131_reduced.pdf` (36 of 71),
`issue17671.pdf` (1 of 10), `issue2537r.pdf` (29 of 60) and `issue7074_reduced.pdf` (7 of 1674).

One of those six is the scan's own artefact and worth stating: `issue2537r.pdf` is the file whose
`indexToLocFormat` is byte-swapped, and the scan read its `loca` under the *stated* format. Under
the format `repaired_loca_format` derives it is monotonic, and that repair runs first. So the
demand is five documents, not six — **and the difference is a reminder that a scan written to
measure one defect inherits every other defect's symptoms.**

## What it moved

`issue11131_reduced.pdf` draws its whole sentence. No gate's headline changes: the page's verdict
was `ambiguous` before and after, because `ghostscript` is 1.6 of 255 away from `poppler` and
`mupdf` on a page 207×41 and there is no consensus to agree with. **That is the point of §3a and
this is the eleventh time it has paid** — a page can be plainly wrong inside this bucket, and
nothing announces the defect or the fix.

Corpus 78 incomplete, oracle 852/68/749, text 98.2%, quorra 913/43/1: all unchanged.

## What is still owed, and it is the reason this was invisible

The page drew half its text **in silence**. `FontError` is the only channel a font has, so a font
that draws *some* of its codes says nothing about the rest — the handover's "a font is reported as
a whole, and that is not fine-grained enough". A report where a *glyph* is missing would have
named this without any of the above, and would need `LoadedFont` to distinguish "this code has no
glyph" from "this code's glyph is blank", which a space legitimately is.
