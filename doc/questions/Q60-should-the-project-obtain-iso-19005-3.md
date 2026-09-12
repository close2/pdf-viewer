# Q60 — Should the project obtain ISO 19005-3, which is the standard's own answer here?

Source: `doc/rfc/0007` §4.7.4, raised by session 954.

## Why it needs the owner

The owner asked to be able to accept an XML attachment while targeting PDF/A-2. The standard
already has a part for exactly that: **PDF/A-3**, whose associated files exist to carry arbitrary
embedded files, and which is what ZUGFeRD and Factur-X specify — the same specification since 2020,
with a German mandate arriving in 2026, embedding `factur-x.xml` with `/AFRelationship
/Alternative`.

`A17` says part 3 was not bought, and the reason was sound when it was given: parts 2 and 4 were
what the work needed. The request changes the calculation, because the departure RFC 0007 §4.7
proposes is **this project routing around a gap of its own making**. With part 3 held, an operator
wanting a PDF with an XML invoice attached asks for a target rather than a departure, and gets a
file that conforms and says so.

Two things make this cheaper than the earlier purchase questions. The validator's shape already
takes a part as a column rather than an implementation (`A46`), and ISO 19005-3 is widely described
as ISO 19005-2 plus relaxed embedding — so most of the requirement table would be shared, as parts
2 and 4 already share most of theirs.

## What the tree does meanwhile

Parts 2 and 4 are targets; part 3 is not, and a non-PDF attachment leaves exactly the two answers
`doc/pdf-a-conversion-limits.md` §3.1 names — retarget to PDF/A-4f, or drop it. RFC 0007 proposes a
third, and this question asks whether the third should instead be a fourth target.

## The correction that sharpens this

The owner noted on 2026-09-11 that part 3 admits **all** attachments, not only the XML the invoice
case needs — the same breadth PDF/A-4f already has. *(Recorded as the owner's reading; part 3 is
not held, so this project has not checked the clause.)*

That matters for what buying it would and would not buy. It answers the ZUGFeRD case completely: a
Factur-X invoice becomes a target rather than a departure, and the file conforms and says so. It
does **not** answer the request that raised it, because "and only xml" is the request, and no part
of ISO 19005 expresses a permission that narrow. Part 3 and PDF/A-4f would each also admit a
spreadsheet or a video.

## Recommendation

Obtain it if the invoice case is real for you — it is the honest answer there, and a conforming
file always beats a departed one. Build departures regardless: they answer the general problem and
part 3 answers one instance of it, and an operator whose archive mandates PDF/A-2 itself, or who
wants a permission narrower than any part grants, has no other route.
