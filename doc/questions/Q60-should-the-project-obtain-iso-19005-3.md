# Q60 — Should the project obtain ISO 19005-3, which is the standard's own answer here?

Source: `doc/rfc/0007` §4.7.4, raised by session 954.
Status: **open** — answered when `A60-should-the-project-obtain-iso-19005-3.md` exists beside this
file.

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

## Recommendation

Obtain it if the invoice case is real for you, and treat the departure mechanism as worth building
regardless — it answers the general problem, and part 3 answers only this instance of it. The two
are not alternatives: an operator whose archive mandates PDF/A-2 specifically is not helped by a
PDF/A-3 file, and departures remain the only route for them.
