# Q855 — May the applicability report print TS 119 172-4's scope text verbatim?

Source: round 2028, building `doc/todo/77` item 3 (ADR 2872). Nothing waits on the answer: the report
says the scope in a paraphrase today, and says that it is one.

## The question

ETSI TS 119 172-4 REQ-4.5-01 a) requires the applicability rules checking report to include a scope
text the clause prints — a heading and one sentence, about sixty words, naming the check as the one
for European qualified electronic signatures and seals using trusted lists under Directive 1999/93/EC
or Regulation (EU) No 910/2014. A report that meets a) prints those words.

ETSI's notice on the document permits no reproduction in any form without its written permission,
and ADR 1085 therefore keeps every ETSI sentence out of this tree: clauses are cited and paraphrased.
Printing a) verbatim means carrying the text as a string in `pdf-signature` or `viewer-core`.

## Options

1. **Ask ETSI for permission** (`editor@etsi.org` handles copyright requests) to carry the two
   sentences as a report string, and carry them once it is given, recorded in
   `doc/third-party-data.md`.
2. **Carry them now**, reading a requirement to print a text as the permission to print it.
3. **Keep the paraphrase**, said to be one, and accept that a) is met in substance only.

## Recommendation

Option 1, with option 3 meanwhile — which is what the tree does today (`verdict::applicability::Scope`
is a type so the verbatim words arrive in one place). Option 2 reads a permission into a notice that
states none, which is the reading ADR 1085 declined.
