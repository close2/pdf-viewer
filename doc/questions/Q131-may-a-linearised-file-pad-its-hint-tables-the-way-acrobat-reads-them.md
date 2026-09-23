# Q131 — May a linearised file pad its hint tables the way Acrobat and qpdf read them?

Source: round 1228, building `optimize --linearize` (ADR 1293).

## Where the annex stands

F.4.1 states the packing in one sentence and its one exception in the next:

> In general, this byte stream shall be treated as a bit stream, high-order bit first, which shall
> then be subdivided into fields of arbitrary width without regard to byte boundaries. However,
> each hint table shall begin at a byte boundary.

`pdf_syntax::linearize` writes exactly that: within one table the fields run on, and only a table
starts on a byte. The clause is not unclear, so principle 5 decides the default and this file does
not ask about it.

## What the evidence says

qpdf, which the owner supplied as evidence, pads **every item's run** to a byte boundary when it
writes and skips to one when it reads, and its source says it does so following Adobe's
implementation notes — that is, following what Acrobat writes and expects. So `qpdf --check` reads
this writer's shared object hint table wrongly and reports group lengths that do not match. Padding
the runs was tried in a scratch build: qpdf then reported no linearisation error on the plain
fixture. The difference is real and it is exactly this one sentence.

The consequence is limited but not nothing. Every reader this tree knows of opens a linearised
file through its cross-reference chain, so nothing *draws* differently. What differs is a reader
that trusts the hint tables to fetch a page by byte range over a slow connection — the use F.2 was
written for — which would ask for the wrong ranges and fall back to reading the file.

## The options

**1. The annex's packing, as built.** Principle 5 as it stands. The cost is the one above: a
byte-range reader following Adobe's convention misreads the tables.

**2. Adobe's convention, as a documented departure.** Pad each item's run. This is A23's shape —
a deliberate departure from principle 5, recorded with its cost — and the cost is that the file no
longer states what F.4.1 says, which a reader following the annex would misread instead.

**3. Both, chosen per output.** A flag; the default decided here.

## Recommendation

Option 1, and keep the difference recorded in ADR 1293 and the F.4.1 ledger row. The annex is
the only definition this project has, byte-range reading is an accelerator whose failure mode is a
slower open rather than a wrong page, and a departure is easier to add later than to take back.
