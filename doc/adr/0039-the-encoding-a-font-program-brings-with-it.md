# ADR 0039 — The encoding a font program brings with it

Status: accepted, 2026-07-30.

## Context

The demand item was `issue20232.pdf`, the oracle's whole `CONTRADICTED_SYMBOLIC_FONT_FLAGS`
bucket: an engineering drawing whose dimension label reads `⌀56` in three references and `56`
here. Its entry blamed a font descriptor with contradictory flags, and the specification track
pointed at the same family — §9.6 was eight `unreviewed` rows and §9.8 was seven more, and
both are what `issue20232.pdf` is about.

So: the same family for both tracks, which is the shape that has worked best.

## What reading the family found

**Three findings, and the demand item is the only one nobody could have predicted from it.**

### 1. §9.8.2 settles a contradiction §9.6.5.4 cannot

`issue20232.pdf`'s descriptor writes `/Flags 36`, which sets the Symbolic bit (3) and the
Nonsymbolic bit (6) at once — a combination Table 121 forbids in the same sentence that
defines each of them. §9.6.5.4's algorithm then has two mutually exclusive branches, one
conditioned on each flag, and reading only that subclause leaves the file undecidable.

§9.8.2 decides it two pages later:

> The use of the two flags to represent a single binary choice is a historical accident. A PDF
> processor should always check the Symbolic flag to determine whether the state is Symbolic
> or NonSymbolic.

Which is exactly what `is_symbolic` does — bit 3 alone. **The code was already right and the
oracle entry's explanation was wrong**, which is this project's sixth-and-seventh instance of
a contradicted page's label naming a hypothesis rather than a diagnosis.

What actually happens on that page is worth writing down, because it is the honest answer:
the font *is* symbolic, `/Encoding` *is* ignored, the code reaches the (3, 0) subtable, and
the glyph it finds there is one of the 158 in that subset with **no outline at all**. Only
glyphs 0 and 90 have any contour, and 90 is the one the `post` table names `Ccedilla` — which
is the name the `/Differences` array gives the code, and the array is the one statement in the
file §9.6.5.4 tells a reader not to read. Three separate "shall"s are broken by that font (the
two flags, a (3, 0) subtable whose codes run to `0x2219` where the clause allows four ranges
none of which contains it, and `/BaseEncoding /StandardEncoding`, which is not one of Table
112's three permitted names), and §9.6.5.4's NOTE 1 says of exactly this situation that
implementations "have evolved heuristics for dealing with such problems; those heuristics are
not described here". The clause's escape hatch — "if a character cannot be mapped in any of
the ways described previously, a PDF processor may supply a mapping of its choosing" — does
not open, because the character *is* mapped. To a blank.

So the page stays listed, with the reading that produces the blank beside it, rather than
being fixed by copying somebody's heuristic. That is principle 5 doing the thing it is for.

### 2. Table 112's default base encoding is the *program's*, for an embedded program

§9.6.5.1's Table 112 gives `/BaseEncoding`'s absence three answers, and only the last two turn
on the Symbolic flag:

> For a font program that is embedded in the PDF file, the default base encoding shall be the
> font program's built-in encoding, as described in 9.6.5, "Character encoding" and further
> elaborated in the subclauses on specific font types.

— and *otherwise*, `StandardEncoding` for a nonsymbolic font and the font's own for a symbolic
one. `simple_code_table`, which is reached only by a font whose bare CFF program was embedded,
asked the flag anyway and gave a nonsymbolic font `StandardEncoding`: the rule for a font this
crate would be *substituting*. So every code the document left to the program drew whatever
`StandardEncoding` puts there.

**Almost nothing, measured over the corpus, and that is the finding rather than an excuse.**
A CFF's built-in encoding usually *is* `StandardEncoding`, so the wrong reading is invisible:
of 974 documents, exactly one font — `endchar.pdf`'s `StoneSans` — has an embedded name-keyed
program, no `/BaseEncoding`, a clear Symbolic flag, and a built-in encoding that differs, and
the difference is one code. This is trap 8's shape exactly: the rule is required of any valid
PDF and only a synthetic test can defend it, so `cff_encoding_tests` states each of the three
cases on a fixture whose two candidate bases disagree in opposite directions, and all three
were confirmed to fail with the old reading restored.

The *names* moved with the glyphs. A code the encoding leaves to the program now takes the
program's charset name for the glyph it selected, which is what a document with no `/ToUnicode`
means by that code — so text extraction and `code_for` gained the codes they had been losing
rather than exchanging one wrong table for another.

### 3. `/MissingWidth`'s default is 0, and half an em was a preference

Table 120 gives `/MissingWidth` the default value 0. Table 109 sends it every code the
document does not declare:

> For character codes outside the range FirstChar to LastChar , the value of MissingWidth from
> the FontDescriptor entry for this font shall be used.

This tree used **half an em**, with a comment arguing that spacing degrades more gracefully
than it does collapsing to zero. That is a plausible thing to want and not a reading of
anything, and it cost a page: `issue7439.pdf` shows character code 2 six times, declares
`/FirstChar 3`, and states no `/MissingWidth`, so six half-ems of invented space opened
between `Issue` and `7439`. The page was contradicted by the reference consensus and now
agrees.

## Decision

- **`simple_code_table` no longer reads the font descriptor at all.** Its signature lost the
  parameter, which is the structural form of the finding: for an embedded program the flag
  decides nothing.
- **`cff::CodeToGlyph::Named` carries `builtin_names`** beside `builtin` — the same mapping
  carried through the charset instead of stopping at the glyph index. Nothing about drawing
  needs it; it exists so that reading the right base encoding does not cost the codes that a
  font with no `/ToUnicode` can only name that way.
- **`DEFAULT_WIDTH` is 0**, with Table 120's sentence above it and `issue7439.pdf` named as
  what it costs.
- **`issue20232.pdf` stays contradicted**, with its entry rewritten from the font's own tables
  rather than from a guess.
- **§9.6 and §9.8 are reviewed as families** — 16 rows, 14 of which were `unreviewed`.

## Consequences

The oracle moved from 758 agreeing and 101 contradicted to **760 and 99**, over the same 1620
pages. Two pages left the contradicted list and **only one of them is a fix**:

- `issue7439.pdf` is one, and it is a width rather than a picture.
- `issue3566.pdf` is **not**. Its raster is byte-identical before and after — checked with a
  digest, not assumed — and what changed is which *bound* it was judged by. Its font is a
  symbolic bare CFF with no `/ToUnicode`, so nothing could name what it drew, so the oracle's
  `has_text` was false and a page that is nothing but the word `different` was held to the
  tolerance measured on flat fills. It passed every absolute bound in that class and failed
  only the relative test. Giving it the program's own glyph names made the readback work and
  moved it to the text tolerance.

That second one is a second witness for a defect the gate already knows it has, recorded in
`CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR`: **`has_text` asks whether we could name what we drew,
and what it means to ask is whether we drew glyphs at all.** Saying so is the difference
between a count that means something and one that improves when the instrument slips.

Two new `silent` rows are the honest cost of reading §9.8.3: `/Style /Panose` and `/FD` are
read by nobody, and while neither can change an *embedded* CIDFont's glyph, both would change
which installed face stands in for one that is not embedded — and nothing says so. The ledger
had one `silent` row and now has three, which is what happens when a family is read rather
than a feature.
