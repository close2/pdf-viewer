# A `loca` table that goes backwards, and half a page of text with it

Status: **diagnosed, measured; not fixed.** Found in the two-hundred-and-twenty-second session.
Priority: 13 — a defect: wrong pixels, in silence
Corpus: `issue11131_reduced.pdf`, and unknown how many others (see "What is not measured")
Clauses: §9.6.3 and §9.7.4.2 name the program; the table is ISO/IEC 14496-22's
Code: `crates/pdf-font/src/lib.rs`, beside `repaired_loca_format`

## The page

`issue11131_reduced.pdf` is 207×41 and draws *Operating Account Consolidated Statement* in an
embedded `CIDFontType2` subset of Arial Unicode MS under `Identity-H`. We draw

```text
p  r ti g   ou t o soli t  St t     t
```

— about half the glyphs, and **report nothing**, because `FontError` is per font and this font
loaded and produced glyphs. Ink: ours 3.29, `hayro` 3.25, `mupdf` 7.92, `poppler` 7.91,
`ghostscript` 9.51. The three renderers that draw the whole sentence are the three that share
`libfreetype`; the two that do not are the two that read the font with `skrifa`.

## What the font says, and it is checkable in twenty lines

The embedded `/FontFile2` is 32 200 bytes, `numGlyphs` 71, `indexToLocFormat` 1, `loca` 288 bytes
— 72 long entries, which is exactly `numGlyphs + 1`. So the table's *shape* is right. Its
contents are not:

```text
loca[0..8] = 16776  16776  16776  16776  10674  2188  2590  1886
```

ISO/IEC 14496-22 requires the offsets to be in ascending order — a glyph's data runs from
`loca[i]` to `loca[i + 1]`, so a descending pair states a **negative length**. **36 of the 71
glyphs have one**, and about half the sentence is missing. The correspondence is the hypothesis
and it is the obvious one: `read-fonts` computes the length from the pair and refuses a negative,
where FreeType parses the entry at `loca[i]` and derives its extent from the entry itself.

**What would settle it in one test**: ask `pdf-font` for an outline for each of the 71 glyph ids
and compare the empty ones with the 36. Worth doing before the fix rather than after.

## Why the fix is not `repaired_loca_format`'s shape

That function repairs a **byte-swapped** `indexToLocFormat` by reading the file's own two
statements of the same fact — the last `loca` entry against `glyf`'s length, and `loca`'s length
against `numGlyphs` — and changing two bytes. Nothing here is recoverable that cheaply:
`loca[i + 1]` is *also* glyph `i + 1`'s start and is used as such, so no single entry can be
rewritten without breaking its neighbour. The glyphs genuinely sit in the `glyf` table in an
order the offsets do not follow.

The repair that is available is a **rebuild**, and it is a derivation rather than a guess for the
same reason: a `glyf` entry is self-describing. `numberOfContours` decides simple or composite;
a simple glyph's extent is `10 + 2 × contours + 2 + instructionLength + flags and coordinates`,
and a composite's is the component loop up to the entry with `MORE_COMPONENTS` clear. So each
glyph's true length can be read from its own bytes, and a new `glyf` in glyph order with a new
monotonic `loca` beside it is exact. Glyph ids do not move, so a composite's references stay
valid.

Roughly eighty lines of parsing, of the fiddly kind — the flag repeats in a simple glyph's
coordinate stream are the part to get wrong — and it belongs beside `repaired_loca_format`,
called from the same place.

## What is not measured

**How many of the 974 documents embed such a font.** The scan is one pass: extract every
`/FontFile2`, read `maxp`, `head` and `loca`, and count the non-monotonic pairs. It has not been
run, so the corpus demand for this is one document and a lower bound.

## What it is worth

One page today, and a class of page: a `loca` in the wrong order is the kind of damage a subsetter
produces, and every renderer built on `FreeType` hides it. **The silence is the worse half** — the
handover's "a font is reported as a whole, and that is not fine-grained enough" is exactly this
page, and a report where a *glyph* is missing would have named it without any of the above.
