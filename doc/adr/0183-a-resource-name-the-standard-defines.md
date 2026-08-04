# ADR 0183 — A resource name the standard itself defines

Status: accepted, 2026-08-04 (session 282).

## The page

`issue17492.pdf` has a text widget whose value is *Several / Other / Jobs*, whose stored
appearance stream draws it, and whose appearance stream says:

```text
154 0 obj << /Subtype /Form /Resources <<>> /BBox [0 0 151.01 87.53] /Length 180>>
/Tx BMC q 1 w 0 G 0 0 151.015 87.53 re S BT /Helvetica 12 Tf 0 g … (Several) Tj … ET Q EMC
```

An **empty** resource dictionary and a `Tf` that names `/Helvetica`. The widget's `/DA` says the
same. This tree drew the box and no text, and reported `no /Font resource named /Helvetica`.

## What the standard says

§7.8.3's third bullet is a `shall` and the file breaks it:

> For other content streams, a PDF writer shall include a Resources entry in the stream's
> dictionary specifying the resource dictionary which contains all the resources used by that
> content stream. This shall apply to content streams that define form XObjects, patterns, and
> annotation appearances.

The fourth bullet — inheritance from the page — is for a `/Resources` that is **omitted**, and
this one is present. So the standard states no answer, which is where a documented choice belongs.

But it does not state *nothing* about this stream, because of what the name is. §9.6.2.2:

> The PostScript language names of 14 Type 1 fonts, known as the standard 14 fonts, are as
> follows: Times-Roman, Helvetica, Courier, Symbol, …

> These fonts, or their font metrics and suitable substitution fonts, shall be available to the
> PDF processor.

A stream naming `/Helvetica` has named something the standard requires this program to have, and
since the hundred-and-forty-eighth session it has it compiled in (ADR 0133).

## Decision

**A `Tf` whose resource dictionary defines nothing under that name loads the standard font where
the name is exactly one of §9.6.2.2's fourteen, and reports as before where it is not.**

`pdf_font::standard::is_standard_name` is the test, and it is deliberately exact — no case
folding, no families, no clones. That is a *different question* from the one
`pdf_font::substitute` asks, and the difference is the whole argument for the narrowness:

| | asks about | may fold, may generalise |
|---|---|---|
| `substitute::names_a_standard_font` | a `/BaseFont` — a typeface's own name, written by whoever made the font | yes: `arial`, `timesnewroman`, case-folded |
| `standard::is_standard_name` | a **resource name** — a label in a dictionary, written by whoever made the file | no: a file may call its resource `/Arial` and mean anything |

**The same argument `variable_text::STANDARD_ABBREVIATIONS` made in the two-hundred-and-fifty-eighth
session, one clause over and with a stronger premise**: there the name is a four-letter convention
denoting one of the fourteen, here it *is* one of the fourteen.

## What it changed

- `issue17492.pdf` draws its three lines and **leaves the incomplete list**: 76 documents → 75.
- The oracle judges it for the first time and it **agrees** with the reference consensus: 854 → 855.
- The text gate's denominator grows by 69 words (22 862/23 279 → 22 931/23 348) and the percentage
  holds at 98.2%.
- Nothing else moves: the quorra comparison, the dates, JPEG 2000 and every unit test are unchanged.

**Two corpus documents naming `/F1` with nothing behind it still report**, which is the narrowness
made visible in a gate rather than argued in a comment.

## The evidence from other renderers, and what it is worth

`mupdf` and `ghostscript` draw the three lines. **`poppler` refuses**, out loud:

```text
Syntax Error: Unknown font tag 'Helvetica'
Syntax Error (69506): No font in show
```

Two and two. Principle 5 says that settles nothing on its own — and here it genuinely did not: the
answer came from §9.6.2.2's `shall`, and the split vote is why quoting it is worth the line. What
it does establish is that this is not a case of four renderers agreeing against us.

## The lesson

**A report can be a clause nobody finished reading.** "No `/Font` resource named X" was a true
statement about the resource dictionary and an incomplete one about the file: the name carried
meaning the resource dictionary did not, and §9.6.2.2 is where that meaning is written down. The
corpus found it by *reporting* — the item was on the incomplete list, one line up from the two
`/F1` documents that look identical and are not.
