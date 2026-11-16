# Q856 — Will you supply ETSI TS 119 312, so that REQ-4.5-01 h) can be assessed?

Source: round 2029, after building `doc/todo/77` item 5 (ADR 2874). Nothing waits on the answer:
the applicability report names the suite a signature used and says that it was not assessed.

## The question

ETSI TS 119 172-4 REQ-4.5-01 h) asks the applicability rules checking report to state potential
security issues of the cryptographic suites a signature used. What decides those is ETSI TS 119 312
(cryptographic suites): its tables say, per algorithm and key length, until when each is
recommended. Round 2028 tried the free download from etsi.org and was answered `403 Forbidden`;
this round was told not to try to get past that refusal, and did not. Without the document the
report can name the suite (ADR 2872) but not assess it.

## Options

1. **You fetch TS 119 312** (the current version, its PDF) through a browser and place it in
   `doc/`, as A713 placed TS 119 172-1 to -4. A round transcribes the tables it needs into a
   `static` table, cited by clause and paraphrased under ADR 1085, and h) is assessed from it.
2. **Leave h) at naming the suite**, said to be unassessed, which is what the tree does today.
3. **Assess against another catalogue** (SOG-IS's agreed mechanisms, or a national one) — a
   document TS 119 172-4 does not name, so the report would answer a different question.

## Recommendation

Option 1, with option 2 meanwhile. The refusal is ETSI's server's, not a charge: the document is
free, and the obstacle is only how it is fetched. Option 3 would be the corpus-going-quiet failure
in better clothes — a source the clause does not cite standing in for the one it does.
