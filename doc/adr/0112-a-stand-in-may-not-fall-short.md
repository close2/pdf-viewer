# ADR 0112 — A stand-in may not fall short

Status: accepted, 2026-08-01.

## Context

The corpus's annotation row has ten reports over ten documents and half of them are one thing:
a `/DA` string naming a font the interactive form dictionary's `/DR` does not define. Five name
`/Helv`, one names `/F1`, one names `/Rufscript`.

§12.7.4.3 puts the requirement on the writer:

> The specified font value shall match a resource name in the Font entry of the default resource
> dictionary (referenced from the DR entry of the interactive form dictionary …).

These documents break it, and the standard states no recovery. This tree refused the whole
construction — and on a free text annotation that is a blank page, because §12.5.6.6 makes the
text the *whole* of the appearance. `freetext_no_appearance.pdf` is 200 lines of PDF whose only
content is one `FreeText` annotation, and it rendered as an empty sheet.

That is ADR 0106's rule one clause family over: **an optional detail must not erase what the
clause requires**, and what this clause requires is the value on the page.

## The decision, and the correction that followed it

A `/DA` font `/DR` does not define gets a stand-in — a synthesised simple font dictionary whose
`/BaseFont` is the resource name — and the report names the font that was missing. Two true
statements instead of one false page.

Passing the resource name through is **a hint rather than a derivation**, and the ADR says so
because the temptation is to claim more: a resource name is arbitrary, `/F1` as often as
`/Helv`, and nothing in the standard says `Helv` means Helvetica. It is handed to `pdf_font`'s
substitution because that is where a name is *ranked* against the other evidence (ADR 0086) and
where an unrecognised one costs nothing — `/F1` matches no family and falls through to the
default, which is the same answer as passing no name at all.

**Then the page was looked at, and the first version of this was wrong.** With the stand-in in
place, `freetext_no_appearance.pdf` drew — a scatter of six dots on an otherwise empty page. Its
`/Contents` is a paragraph of Arabic in UTF-16BE, and a Latin stand-in has codes for its spaces
and full stops and for nothing else. That is trap 1's archetype exactly: a metric said the page
was better and the picture said it was garbage, and garbage is worse than the blank it replaced.

So the rule is asymmetric, and the asymmetry is the finding:

- Where the **document** names the font, a character the font lacks is reported and the rest is
  drawn. The shortfall is the document's own choice and drawing what it can is honest.
- Where **this crate** invented the font, a single character it cannot address declines the
  whole thing. An invention that cannot show what the document says has no claim to be on the
  page at all.

## Consequences, measured

Of the six documents, **two now draw their fields and four keep their blank**. `issue19389.pdf`
shows "Password Field: ····················" and "Text Field: This should be visible", which is
what the file is named for; `poppler-395-0-fuzzed.pdf` draws its two signature annotations. The
four that keep the blank are the Arabic ones, declined by the rule above.

**All four gates are unmoved** — 89 incomplete, 841 agreeing and 65 contradicted, 97.9% of
`pdftotext`'s words, 1545 dates — and that is the intended shape: the documents keep their
reports, so they stay in the incomplete count and out of the oracle's judged set. What changed
is on the page, and the page is the only instrument that could see it. Tests 890 → 891, one for
each half of the asymmetry.

One detail worth carrying: the constructed stream says `/{name} {size} Tf`, so the stand-in has
to arrive in the appearance's `/Resources` with it, or the interpreter would report a missing
resource instead of the missing definition. `with_stand_in_font` is that, and `/DR`'s own entry
always wins, because there is only a stand-in where `/DR` had none.

## What this does not license

Substituting wherever a resource is missing. The argument turns on §12.5.6.6 making the text the
annotation's whole appearance, so that refusing draws *nothing* — and on the stand-in being
checked against the value before it is used. A substitution that is not checked is the "helpfully
fall back to a default that renders something plausible" trap 5 forbids by name, and this session
built exactly that before the picture rejected it.
