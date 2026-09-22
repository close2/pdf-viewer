# Q97 — May we acquire ISO 19444-1 (sections 6.4 and 6.6) for XFDF annotations?

Source: round 1193, building §12.7.8.3.4's FDF annotation import.

## What this round settled, so that the question is narrow

§12.7.8.3.4's own requirement is one sentence, and ISO 32000-2 states all of it: an FDF
annotation dictionary carries Table 254's `/Page`, and everything else in it is clause 12.5's,
read by the clauses that read any annotation. That import is **built** this round — the
annotation is copied out of the FDF file's object space and placed on the page its ordinal names
(ADRs 1223, 1224). ISO 19444-1 was not needed for any of it, and the ledger row said it might be.

What ISO 19444-1 defines is the **XML spelling** of the same data — XFDF — and that is where this
tree still stops. `pdf_model::xfdf` reads a file's fields, because ISO 19444-1:2019 sections
5.5.2, 5.6.2 and 5.6.3 are inside the preview this tree holds (`doc/md/ISO-19444-1-2019-preview.md`,
which ends at section 5.7.1). Its **annotations** are counted, named on `FormsData::owed` and read
by nothing, because the elements and attributes that spell them are that standard's **sections 6.4
and 6.6**, which the preview does not reach.

## Why a round cannot close it

Principle 5. Building the element and attribute mapping from another reader's output is the thing
the principle forbids outright, and here it would fail worse than a refusal does: a wrong
attribute mapping draws a plausible annotation in the wrong place on the page, which no gate in
this tree can see. The alternative to the text is not a careful guess; it is the refusal that
stands today.

## What it would cost

ISO 19444-1:2019 (*Document management — XML Forms Data Format — Part 1: Use of ISO 32000-2
(XFDF 3.0)*, second edition) is sold by ISO as catalogue number 68836 and by the national bodies;
`doc/third-party-data.md` records that sis.se is where this tree's preview came from, so the
seller is one already used. **This round did not manage to read a figure off either catalogue
page** — both answer a page whose price is filled in by script — so the price is the one the
owner will see at `https://www.iso.org/standard/68836.html`, a single purchase rather than a
subscription, and this question deliberately states no number it could not check.

It is the same document this tree already holds a preview of, so nothing about how it would be
stored or cited changes: ADR 0187's position applies, it goes under the ignored `doc/md/`, and it
is **cited and paraphrased, never quoted**, exactly as sections 5.5.2 and 5.6.2 are today.

## What it buys

- §12.7.8.3.4's row loses its one departure; it is `departed` this round for that reason alone.
- §12.7.6.2's Table 240 bit 6 submission already writes XFDF *fields*; the round that wrote it
  left annotations out on the same evidence, so the two halves close together.
- `doc/questions/A23` already departs from principle 5 deliberately for one XFDF question; this
  is the purchase that makes that departure unnecessary rather than permanent.

## Recommendation

**Buy it.** The sum is small against a whole format that two ledger rows are blocked on, the
tree already holds and cites the preview so nothing about the licensing position is new, and the
alternative is not a cheaper answer but a permanent one: an XFDF file's annotations go on being
counted and refused for as long as the text is absent.

If the answer is no, say so and the two rows record it as a decided exclusion rather than a
blocker — which is a better state than the one they are in now, because an exclusion does not
decay the way a "we have not got the text yet" does.
