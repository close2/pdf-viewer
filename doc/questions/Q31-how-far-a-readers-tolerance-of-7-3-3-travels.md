# Q31 — How far should a reader's tolerance of §7.3.3's writer-side error travel?

Raised by session 932, out of `corpus-cache/tika-issue-tracker/batch5/qpdf`'s ink ranking. The
reading is [ADR 0904](../adr/0904-a-dimension-written-as-a-real.md); this is the half that is a
policy about the whole tree rather than about one clause.

## The question

§7.3.3 says

> A real number shall not be present when an integer is expected.

That `shall not` is addressed to whoever writes the file. The standard says **nothing** about what
a reader does with a file that breaks it, so every answer is a choice. This tree has now made the
same choice twice, in two files, on two different entries:

- ADR 0371, §7.10.5's calculator: an operand typed `int` reached by a real is **truncated**;
- ADR 0904, Table 87's `/Width` and `/Height`: a dimension written as a real is **truncated**.

There are dozens of other entries a table types as an integer and this tree reads with
`Object::as_integer`, which answers `None` for a real: `/BitsPerComponent`, Table 11's `/Rows`,
`/Columns` and `/K`, `/Count`, `/Rotate`, `/N`, `/StructParent`, `/Length`, and so on. **Should
the rule travel to all of them, and if so, where should it live?**

Three positions, each with a real cost:

1. **Case by case, as a measured population demands it** — where the tree is now. Every tolerance
   is exercised by a document that exists, so nothing untested ships; the cost is that the next
   such document costs a round, and that two readers of one file can disagree in the interval (the
   defect ADR 0904 found: five call sites in one module, four of which would have read the same
   real `/Width` as zero).
2. **One accessor, applied everywhere at once** — a documented `Object` method beside `as_integer`
   and `as_number`, used wherever a table types an entry as an integer. One rule, one place, and
   the tree stops being able to disagree with itself; the cost is a great deal of behaviour no
   document has ever exercised, which is exactly what principle 1 calls a shortcut taken silently.
3. **At the lexer**, reading `1062.00` as `Object::Integer(1062)`. It would make the tolerance free
   everywhere — and it is the one this session thinks is wrong, because it destroys the
   *distinction* the clause draws: `Object::Real` is how this tree knows a file broke a `shall`,
   and §7.3.3's own row records three earlier departures that all kept the token's identity.

## Why it cannot be settled without you

It is not a clause reading — there is no sentence to read, which is the whole point. It is a
question about how far a documented departure travels before it stops being a departure and starts
being this program's number grammar, and about which of principle 1 ("no shortcut taken silently")
and principle 3's robustness ("every PDF that exists renders as its producer specified") gives way
where they pull apart. The measurement does not settle it either, and this session took it: over
**89 256 documents** — every corpus on this disk — exactly **two** carry a real where an integer
was expected, and **zero** of the crawl's 65 944 do. That is an argument for position 1 and it is
also an argument that position 2 would cost nothing to be wrong about. It is your call which.

## What the tree does meanwhile

**Nothing is blocked, and the answer would only widen what already works.** `/Width` and `/Height`
truncate; every other integer entry refuses a real exactly as it did, loudly, through the report
its own reader already makes. `pdf_model::image::dimension_entry` is one function of nine lines, so
position 2 is a mechanical change the day it is wanted, and position 1 costs nothing to stay in.

## Recommendation

**Position 1, with a standing rule written down**: the tolerance goes where a measured population
puts it, and wherever it goes it goes into *one* function for that entry rather than into each call
site. The second half is the part this session would like ratified, because it is what the defect
actually was — not that `/Width` refused a real, but that five readers of `/Width` in one module
would have answered differently once one of them stopped.
