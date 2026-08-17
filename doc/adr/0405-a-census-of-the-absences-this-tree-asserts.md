# ADR 0405 — A census of the absences this tree asserts, and the five that were not

Status: accepted.
Session: the five-hundred-and-seventieth, a general improvement round.

## 1. What this decides

ADR 0403 found one ledger row resting on a `grep` that had classed a corpus of PDFs as binary and
printed nothing. It left a question rather than a conclusion: **how many others are there?** This
round answers it.

1. **Every "no corpus document does X" sentence in the tree is a claim that decays**, and about 165
   of them were live. **Five were false.** They are corrected where they are written.
2. **`grep -a` is not the fix, only half of it.** A PDF states most of its names inside deflated
   object streams, which no byte search over the file can reach with or without `-a`. The census
   this round builds walks the *objects the cross-reference table names*, and four of the twelve
   `/IDTree` witnesses are visible only that way.
3. **A claim of absence has a population, and this tree's sentences did not say which.** Two of the
   five falsifications are nothing but that: true of pdf.js, written about "the corpus", and false
   of the corpus this project actually measures over.
4. **Two clauses acquire producers' files for the first time** — §12.4.3's articles and §12.3.5's
   collections — and both readers turn out to be right about them.

## 2. The instrument, and why there are two of it

A census of absences is only as good as its tool, which is ADR 0403's whole lesson; re-using the
suspect tool would have learned nothing. So neither half of this one is a grep.

**`examples/witness_census`** asks a name three times of each of the 1251 PDFs under
`doc/pdf.js`, `doc/corpora/` and `doc/corpora-own/`:

| layer | what it reads | what it misses |
|---|---|---|
| `raw` | the file's own bytes — exactly what `grep -a` reads | anything compressed |
| `objects` | every object `xref().object_numbers()` names, walked as `Name` tokens, **object streams included** | a name that appears only in stream *data* |
| `streams` | every stream's decoded data | nothing, but it matches text rather than syntax |

`objects` is the column to believe: it matches a name as a *token*, so `/Lock` does not match the
word `Locked` and a name inside a string does not match at all. The other two are printed beside it
so that the gap is a number rather than an argument. It is a **lower** bound — twelve of the 1251
do not open without a password, and it assumes generation 0 — which is the right direction for
falsifying an absence.

**`examples/absence_audit`** re-asks seven of the written claims through the readers that would act
on them: `Articles::read`, `Collection::read`, `Viewports::read`, `signature::field_mdp`, the
`/IDTree` resolved as §7.9.6's name tree. A name being *present* is not the structure being
present, and only the second decides a claim like "no corpus document states an article".

Running both is ADR 0403's rule, and here it pointed the other way: there the byte search lied and
the reader was right; here the byte search is the weaker instrument on every term.

## 3. What was false

### §14.7.2's `/IDTree` — a correction that stopped at the file it was written in

Three places said "no corpus document has an `/IDTree` at all — the 89 tagged ones state none":
`destination.rs`'s fixture comment, `viewer-core/tests/fragments.rs`, and §O.3's ledger row.
**Twelve of the 974 state one**, holding between 1 and 285 identifiers, and four of the twelve are
inside object streams where no byte search sees them.

The sharpest part is that this tree already knew. `Tree::element_by_id`'s own doc comment carries
the correction — "this line used to say 89 of the corpus's 89 tagged ones state none, and it was
wrong from the round that wrote it: 12 of those 89 state one" — put there by the round that built
`cell_header_census`. That round corrected the sentence in front of it and left three copies
standing, one of them in the same crate. `doc/habits.md`'s "a retired claim is a string, and
strings are greppable" has no larger instance in this project.

### §14.10.2's `/SpiderInfo` — one ledger contradicting itself

§14.10's row said "[n]o corpus document writes a `/SpiderInfo`". §7.7.2's row, in the same
`ledger.toml`, lists among the catalog entries it does not read: "`/SpiderInfo` (§14.10's web
capture, **5 documents**)". Both sentences were written from a measurement and only one of the
measurements was taken. Five of the 974 state one on their catalog, and two more outside pdf.js.

Nothing is owed here — the clause is deprecated by ISO 32000-2 itself and every structure in it is
a capturing application's registry — so the omission and its reason are unchanged. What is gone is
the false half of the reason.

### §8.6.5.6's default colour spaces — a count that was wrong by nine

"One corpus document names a default at all and its images state their space directly, so nothing
in the corpus could have found it", in the ledger and in `colour_paths.rs`. **Nine of the 974 name
one** — eight a `/DefaultRGB`, `bug886717.pdf` a `/DefaultCMYK` — and `bug1019475_1.pdf` states its
inside an object stream.

The fixture is still right to exist, and the reason changes from a count to an argument: a
default's effect on an image is only visible where the image names a *device* space rather than
its own, which the fixture is by construction and which no corpus document has been shown to be.

### §12.4.3's articles and §12.3.5's collections — the population, not the count

These two are a different failure and the more interesting one. Both sentences were **true** of the
974 documents in `doc/pdf.js`, and both were written as "no corpus document".

The corpus this project measures over is wider than pdf.js, and every other absence claim in the
tree says so — `signatures.rs` measures `/Lock` "over the pdf.js corpus and the four under
`doc/corpora/`", `cell_header_census` runs over 1251 files, ADR 0403's own population is all three.
Over that population:

- **Four documents state an article, with 115 beads between them.** Two of them are named for the
  fact: `PDFBOX-3110-poems-beads.pdf` and its cropbox twin lay two poems out as two threads apiece.
  `format-corpus`'s `443752.pdf` carries one thread of **ninety-one** beads.
- **One document is a portable collection**: `pdfCabinetOfHorrors/digitally_signed_3D_Portfolio.pdf`,
  eight schema columns, `/View /Navigator`, a `/Folders` tree, five embedded files filed under it.

A sentence that does not name its population cannot be checked by reading it, which is why this one
survived: every round that read it read a true statement about a corpus, and a different one.

## 4. What the witnesses settled

Both readers are right about them, which is worth as much as a defect would have been — these are
two of the clauses `CLAUDE.md`'s trap 8 names as written from the standard alone.

`Articles::read` walks `PDFBOX-3110-poems-beads.pdf` to two threads titled `Erlkönig` and
`Moulière` — §7.9.2.2 text strings out of Table 162's `/I`, the first of them settling that the
decoding is not byte-for-byte — with all eleven beads naming their page and each ring closing on
its own first bead after exactly as many `/N` steps as it has beads. That last is the assertion a
hand-built fixture cannot make honestly: a fixture written beside the reader closes because its
author closed it.

`Collection::read` reads the portfolio's eight columns and its folder tree, and `folder_of` splits
all five `/EmbeddedFiles` keys — `<0>Design of Landing Gear.pdf` and four siblings — into §12.3.5.2's
folder number and file name. The same objection applies and is answered: a producer's own keys are
not this reader's keys.

`pdf-model/tests/{articles,collections}.rs` pin both, skipping where the optional submodule is
absent and panicking where it is present and wrong, which is `doc/habits.md`'s rule.

## 5. What was sound, and it is most of it

The terms re-measured and confirmed absent, over the population each claim names: `/Lock`,
`/Trans`, `/Dur`, `/PresSteps`, `/Hide`, `/Legal`, `/TrapNet`, `/DPartRoot`, `/FL`, `/PV`, `/PI`,
`/Requirements`, `/ReversedChars`, `/Alternates`, `/RF`, `/NoRotate`, `/CalCMYK`, `Adobe.PubSec`,
§12.6.4.7's `/Thread` action, §12.7.6.4's `ImportData`, §14.8.5.6's `/Checked`, §14.11.3's
`/MarkStyle`, §12.2's four boundary entries, every PAdES sub-filter and `DocTimeStamp`,
`MacExpertEncoding`, §8.11.4.4's `/Zoom`, `/User` and `/Language` usage categories.

**One deserves its own line, because a name census would have called it false and it is not.**
§12.9.2's number-format algorithm is recorded as having "no corpus witness at all", and one
document does state a `/VP`. The row is right: `bug1146106.pdf`'s viewport is `GEO`, §12.10's
geospatial dictionary, and §12.9.2's arithmetic is `RL`'s. `measurement.rs` says so in as many
words, three paragraphs above the claim. **A claim that survives a census taken at the wrong
granularity is the reason `absence_audit` asks structures rather than names.**

## 6. Why this is not a gate

`doc/todo/01` gains a sixteenth sweep rather than `tools/conformance` gaining a check, and the
reason is the one ADR 0403 gave: the failure is reading rather than enforcement.

A checker over these sentences would have to decide, from prose, which population and which entry
each is about. The one machine-checkable shape here — a row asserting an absence while another row
counts the thing — is a cross-row semantic comparison in natural language, and a gate that fired on
every "no corpus document" would fire about 165 times for five findings. `doc/habits.md` already
holds what such a gate is worth.

What makes the failure not recur is instead that re-checking is now cheap and named: two commands
in `doc/verify.md`, a sweep in `doc/todo/01` that says which three shapes to look for, and
`--names`, which turns "is there a witness for this entry" into a lookup rather than a run.

## 7. What this does not claim

- **The five are not five bugs.** Four of them cost nothing but a wrong reason; the fifth —
  §12.4.3 and §12.3.5's populations — cost two clauses their only real evidence for as long as the
  sentences stood, and both readers turned out to be right anyway.
- **The census is a lower bound.** Twelve of the 1251 files do not open, generation numbers other
  than zero are not asked for, and the `objects` layer cannot see a name a document states only
  inside a content stream. Each of those makes it under-report, which is the safe direction.
- **`doc/corpora/format-corpus` is checked out sparsely here**, and the `jhove-errors` subtree ADR
  0403's second `FieldMDP` witness came from is not in it. So the FieldMDP population this round
  measures is one document where that ADR names two, and the difference is the checkout rather
  than the corpus.
- **Nothing in `doc/adr/` or `doc/history/` was corrected.** Those are records of what a round
  found, and a false sentence in one is part of the record; the correction belongs in the live
  documents and in this file.
