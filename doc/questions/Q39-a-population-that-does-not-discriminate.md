# Q39 — Is a population that cannot discriminate a population?

Raised by session 936's sweep of §7.3.3's reader side ([ADR 0912](../adr/0912-five-answers-to-one-clause.md),
[ADR 0913](../adr/0913-one-place-for-the-rule.md)). It is the half of that sweep this round could
not settle by argument or by measurement, and it is one step past
[`Q31`](Q31-how-far-a-readers-tolerance-of-7-3-3-travels.md) rather than a restatement of it.

## How this relates to Q31

`Q31` asks **how far** a reader's tolerance of §7.3.3's writer-side error should travel — case by
case as a measured population demands it (position 1, where the tree is), one accessor applied
everywhere (position 2), or at the lexer (position 3, which that file argues against). Its
recommendation is position 1 *plus* a standing rule about shape, and session 936 took both: the
rule now lives in one function (`pdf_model::integer_entry`), and it answers for the entries a
measured population names and no others.

Taking position 1 seriously is what raised this question, because **"a measured population" turns
out to have two readings, and until this round nothing separated them**:

- **a document exists that writes the malformed value**, or
- **a document exists whose page comes out differently depending on the answer.**

For `/Width` and `/Height` the two readings agree — the witness that writes `/W 1062.00` is the
witness that drew a blank sheet — so ADR 0904 never had to choose between them. `/Rotate` is the
first entry where they come apart.

## The question

Table 31 types `/Rotate` an integer and calls it "[t]he number of degrees by which the page shall
be rotated clockwise when displayed or printed. The value shall be a multiple of 90. Default value:
0". `pdf_model::page` reads it with `as_integer`, so a real is refused and the page keeps whatever
it inherited.

Over 90 128 documents, **nine entries in three documents** write it as a real. Not one of them
draws differently either way:

- `5220319.pdf` writes `/Rotate .00` seven times, beside one page that writes the integer `0`, and
  0 is the entry's own default;
- `PDFIUM-984-0.pdf` writes `/Rotate .00` and states the entry nowhere else, so the default answers;
- `PDFIUM-984-1.pdf` writes `/Rotate 270.00` on its page — and its `/Type /Pages` parent writes
  `/Rotate 270` as an integer, so §7.7.3.4's inheritance supplies the very value the page's own
  entry could not. The page draws rotated today, by an accident of the file's redundancy.

So: **does the tolerance go to `/Rotate`?** The population is real by the first reading and empty
by the second, and the two answers have different costs.

- **Reading it** (the nearest integer, as for a dimension) makes the page's own words decide.
  `/Rotate` is a magnitude with a stated constraint — the same table says the value "shall be a
  multiple of 90" — so a bad rounding is caught by the same check that catches a bad integer, which
  is family 1's argument in ADR 0912. And it is the only entry in the whole measured population where
  a wrong answer is visible as *the entire page*: a landscape sheet shown portrait.
- **Refusing it** keeps the tolerance where a document has actually needed it, which is what ADR
  0904 chose deliberately: "a tolerance no document exercises is untested code rather than
  robustness". And the refusal is not a default here — it falls through to the *inherited* value,
  which is a reading of the file rather than an invention, and which happens to be right on the one
  document that could have shown a difference.

## Why it cannot be settled without you

It is not a clause reading: §7.3.3 says nothing to a reader, and §7.7.3.3 says nothing about a
malformed `/Rotate`. It is not a measurement either, and that is the whole point — this round took
the measurement and it came back **unable to decide**, which is a result rather than a gap. What is
left is a policy about what counts as evidence for widening a documented departure, and it will
recur: `/FormType`, `/Order`, `/OPM` and `/Count` are all entries where a real exists in the world
and no page moves.

`CLAUDE.md`'s two principles pull opposite ways here exactly as `Q31` says they do. Principle 1
forbids shipping behaviour nothing exercises; the definition of *done* — "every PDF that exists
renders as its producer specified" — asks for the page the producer asked for whether or not a
corpus happens to hold a file that proves it.

## What the tree does meanwhile

**Nothing is blocked.** `/Rotate` refuses a real exactly as it did, and inheritance answers where
the file states the value twice. `/Width` and `/Height` read one; every other integer entry
refuses, loudly where it reports at all. Extending `/Rotate` is one line and one call through
`pdf_model::integer_entry::dimension`'s neighbour the day the answer comes.

## Recommendation

**Refuse — keep the second reading, and write it down as the rule.** A tolerance is a departure
from what the standard types, and a departure earns its place by mending a page somebody can see.
"A document writes it" is a weaker test that would, applied consistently, widen the rule to every
one of the 117 key names the model types integer, which is `Q31`'s position 2 arrived at by
attrition instead of by decision.

The cost of that recommendation is stated rather than hidden: it means this program draws
`PDFIUM-984-1.pdf` right for a reason the file did not have to give it, and a file writing
`/Rotate 90.0` and nothing else would be shown unrotated. If that is the wrong trade, the answer is
the first reading and the rule should say so in those words — *a population is a document that
writes the value* — because the next four entries are waiting behind it.
