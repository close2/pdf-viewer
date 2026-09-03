# 0852 — The fifth writer shown to another reader, and a refusal that said the file defeated us

Session 905. Status: **accepted**. RFC 0002 §9's fourth layer, completed; and RFC 0002 §4.4's
exit statuses, corrected on the three refusals that had the wrong one.

## Context

ADR 0839 built the foreign readback over four writers — `attachments --attach`, `split`, `merge`
and `pages`. ADR 0842 then added a fifth, `optimize`, which landed on a branch while that walk was
being written, so the walk knew nothing about it. Session 900's own record named the gap and said
why it mattered more than a fifth of the work: the other four carry a producer's objects into a
new file with their numbering, their filters and their object-stream membership mostly as they
were, and `optimize` is the verb whose whole *point* is that the bytes are different. Every object
is renumbered, §7.5.7 packs what it permits into object streams, §7.5.8's cross-reference stream
replaces the table, and §7.4's encoding of every stream may be rewritten.

That is also why it is the output this project's own instruments are least placed to judge. ADR
0843 §2 is the standing evidence: a recompressed image whose `/DecodeParms` had been rebuilt in
the **source's** numbering left every raster in that run bit-identical while the file was corrupt.
A misreading on the way out and the matching misreading on the way back in agree with each other,
and only somebody else's reader breaks the tie.

## Decision

### 1. `optimize` joins the walk with the shipped defaults, not a configuration of the walk's own

`crates/pdf-transform/tests/foreign_corpus.rs`. The fifth lane writes what
`pdf-transform optimize` writes for a person — `prune`, `ObjectStreams::DEFAULT` and
`Streams::DEFAULT` — because object streams and recompression are exactly the two passes another
reader may decline, and a walk that turned them off to be safe would be measuring a configuration
nobody runs. That is `doc/todo/02` §2's own lesson about a gate that turns a shipped setting off,
one directory over.

Nothing else about the walk changes: the comparison stays foreign-to-foreign, page 1 of the
rewritten document against the *same* reader's page 1 of the source, because `optimize` leaves the
page order alone.

### 2. qpdf's verdict is a fall of one step or of two, and only one of them could be said

qpdf answers 0 for a sound file, 3 with warnings and 2 with errors. ADR 0839 read the *change* in
that verdict rather than the verdict, which is right — a corpus document qpdf already complains
about says nothing about what we wrote — but it collapsed the two possible falls into "no worse".
So a file qpdf had nothing to say about becoming one it warns about was unsayable, and that is the
smaller signal `optimize` is likeliest to produce, for the three reasons above.

`qpdf gained a warning` is now its own counted, printed and asserted lane. It is empty for all five
writers over the walk's 203-document sample, which is what makes the assertion honest rather than
aspirational (trap 11: a report is only as good as the condition it fires on).

### 3. What the readers said

Over the walk's sample, `optimize` writes 202 derived files and refuses 1 by name.

| | attach | split | merge | pages | optimize |
|---|---|---|---|---|---|
| written | 196 | 202 | 202 | 21 | **202** |
| qpdf held | 196 | 202 | 202 | 21 | **202** |
| qpdf gained a warning | 0 | 0 | 0 | 0 | **0** |
| poppler identical | 194 | 200 | 200 | 21 | **200** |
| mupdf identical | 195 | 198 | 195 | 21 | **198** |
| §14.7 shapes agreed | 75 | 74 | 74 | 10 | **76** |
| §14.7 faults | 0 | 2 | 2 | 1 | **0** |
| drew differently | 0 | 0 | 3 (held) | 0 | **0** |

The two *identical* rows move by one or two between runs and the reason is the walk's own: a
foreign reader that outruns the walk's 20-second budget on the **source** takes that document out of its
comparison, so those two rows count a page rather than measure one, and only the rows that cannot
move — the faults, the differences and the warnings — carry a verdict.

**No defect was found, and the two rows worth reading are the last two.** `optimize` agrees with
the source on more parent trees than any other verb and states no §14.7 fault at all, where `split`
and `merge` each state two and `pages` one — which is the shape the clauses predict rather than a
surprise: the faults those three carry are elements the *source's* parent tree names and whose
hierarchy reaches nothing of them (ADR 0839's `STRUCTURE_HELD`), and a verb that carries the whole
document has no piece to leave them out of.

**No older reader declined an object-stream file.** That was the risk this lane was added for —
§7.5.7 and §7.5.8 are 1.5 constructs and this is the first thing this project writes that uses
them — and the answer from the three installed readers is that none of them noticed. qpdf 12.4.1,
poppler 26.08.0 and mupdf 1.28.0 all read them; the claim is about those three at those versions
and about nothing else, which is what a foreign readback can honestly say.

**And the direction of qpdf's verdict, over the whole corpus rather than the sample**, because the
walk's stride-8 sample is a bound on wall clock and a claim of absence is refuted by one witness.
955 of `doc/pdf.js`'s 974 documents are rewritten; the other 19 are refused, 9 for a password, 1
for an encryption this tree does not open, and 9 as §C.4 reconstructions. Not one file's verdict
got worse. 243 went from warnings to clean and 7
from *errors* to clean; 22 stayed at warnings and 6 at errors. So on this corpus `optimize` is
something qpdf likes better 250 times and likes less never — which is evidence about the reading
(principle 5) and not a target: what makes it good news is that a rewrite states plainly what the
source stated by recovery, and §7.5.5's own row now says so.

### 4. A refusal that said the file defeated us, when the file did nothing of the kind

Found while reading the exit statuses that scan produced, and it is a defect against this project's
own document. RFC 0002 §4.4:

> 2 means the *file* defeated us, 4 means *we* declined, and a caller scripting the suite can tell
> them apart without parsing stderr.

Session 900 refused three constructions by name and its own record called them "[r]efused by name,
which is trap 5". They were `Refusal::Assembly`, whose status is **2**. All three are documents
this tree opens, pages and draws — `pdf_model::Pages` finds what Table 31 describes, which is right
for a *reader* — and what declines is the **writer**: Table 15's `/Root` and Table 29's `/Pages`
are each "( Required; shall be an indirect reference )", so a rewrite of a §C.4 reconstruction
would state a structure no producer wrote. Nothing about those files defeated anybody.

`Refusal::Reconstructed` is that classification, carrying `Exit::Refused`. `Refusal::Assembly`
keeps its other members and its status: a ceiling one file cannot state and a numbering that is
spent genuinely are the file defeating the writer.

Nine corpus documents change status, each 2 before and 4 after: `Brotli-Prototype-FileA.pdf`,
`bug1020226.pdf`, `issue19484_1.pdf`, `issue19484_2.pdf`, `issue9105_other.pdf`, `issue9418.pdf`,
`poppler-395-0-fuzzed.pdf`, `poppler-742-0-fuzzed.pdf` and `REDHAT-1531897-0.pdf` — four on Table
15's `/Root` and five on Table 29's `/Pages`. The ten refusals that are *not* this — nine for a
password, one for an encryption this tree does not open — correctly stay at 2. That is trap 13's
discipline: the change was run against the condition before it was believed, on real documents,
and in both directions.

**The general lesson is the ledger's, wearing a CLI's hat.** The refusal's own message, the ADR
that introduced it and the round's history file all said *refused by name*; the code said
otherwise, and no gate compared the two because the exit status is the one thing none of the five
walks reads. `tests/optimize_corpus.rs`'s census now prints each refusal's status beside its
message, so the corpus itself says which side of §4.4's line every refused document falls on.

## Consequences

- The suite's five writers are all read by somebody else. `doc/todo/57` §5's list of what the
  readback does not cover is unchanged: it is a sample, it draws page 1 only, it skips a document
  needing a password, and it says nothing about the outline, the name trees or the form.
- A caller scripting `optimize` can now distinguish "this file is not one I will rewrite" from
  "this file defeated me" without parsing stderr, which is what §4.4 exists for.
- The claim "no installed reader declines an object stream we wrote" is about three programs at
  three versions and decays the way any such claim does; it is the walk that keeps it current, not
  this paragraph.
