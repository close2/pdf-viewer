# ADR 0758 — The corpus a sentence counted over, made into an instrument

Status: accepted, 2026-08-29. Session 830. Cites ISO 32000-2 §12.8.2.2, §12.8.3.3.2 and §12.8.5
for the three rows this round's own sweep sent it back to; no clause status moves, because nothing
here changes what this program does with any of them. It follows ADRs 0751, 0753 and 0754, which
each found the same defect by accident, and sits beside 0709 (the twenty-second sweep, whose shape
this one copies) and 0397 (a sweep whose right-hand side is a constant measures the day it was
written).

## The class, and why four accidents make one

Four consecutive rounds found the same defect, and not one of them was looking for it:

| round | where | what widening it did |
|---|---|---|
| 821 (ADR 0751) | `fuzz/seed_x509.py`'s stated population — one corpus submodule | 22 certificates became 941 |
| 825 (ADR 0754) | `doc/verify.md`'s `cms` recipe, the identical defect one target down | and the census it then ran **falsified two written sentences** |
| 824 (ADR 0753) | a census whose every corpus witness was of one shape | a defect no gate could see |
| 828 | `doc/todo/01`'s two record sentences about §12.8.5, one file over | written down, because a merge round does no feature work |

The shape they share is one sentence: **a claim counted over a corpus, in a tree that now holds a
population two orders of magnitude wider than the one the claim was counted over.** Every one of
those sentences was *true when it was written*. What moved is the tree — `doc/pdf.js/test/pdfs`
is what this project has meant by *the corpus* since its first rounds, and the SafeDocs crawl
beside it is sixty-six times its size — and nothing in the tree was in a position to say which
sentences the growth had invalidated.

Four accidents are a class, and a class that is found by accident is found at the rate accidents
happen. So this round turned it into an instrument.

## The predicate, and what was hard about it

**A sentence quantifies over a corpus, and does not say which corpus.** Getting that to be
*decidable* is most of the work, and both halves needed an argument:

- **Quantifies over a corpus.** One of `no`, `zero`, `neither` (an absence), `one`, `only`, `sole`
  (a uniqueness) or a cardinal governing `corpus`, `document` or `file`, in a sentence that
  mentions a corpus at all.
- **Says which corpus.** The sentence names a population: a corpus's own directory name, one of
  this project's words for a population that no directory spells (`crawl`, `curated`, `submodule`,
  "both populations"), or a numeral denominator.

**The right-hand side is the disk.** `Populations::read` counts the PDFs under
`doc/pdf.js/test/pdfs`, under each directory of `doc/corpora/` and under each of `corpus-cache/`.
That is ADR 0397's rule and 0709's practice: a sweep whose answer is written into the program
measures the session that wrote it. It costs ten seconds where every other sweep here costs a
fraction of one, and the ten seconds are the walk. It is also a check on itself — the walk's total
is the same 67 460 that `find -L doc/pdf.js/test/pdfs doc/corpora corpus-cache -name '*.pdf'`
prints and that ADR 0754's harvest ran over.

### Where it departs from the twenty-second sweep, and why the calibration decided it

ADR 0709's `parts` refuses to read across a modifier: "both **native** hosts" counts two of three
hosts and claims nothing about how many hosts there are, so adjacency is what keeps a subset claim
out of its report.

**Here the same rule would have missed one of the four findings.** `doc/verify.md`'s `cms` recipe
said "the eleven `/Contents` blobs the **nine signed corpus documents** hold" — and a modifier
narrows *what* is counted while leaving the population it is counted over exactly where it was,
which is the question this sweep asks. So it reads across at most three words, never past a
function word (`more`, `than`, `are`, `from` …) or a unit (`96 MB document` counts megabytes), and
through a partitive (`one of the corpus documents`).

That is a departure taken **on evidence rather than on taste**: it was made after the calibration
below showed the tighter rule silent on a live defect, not before.

### The direction it is loose in is the direction that hides a case

Denomination *removes* a hit. So a name that answers by accident is the one way this sweep can go
quiet about a claim, and two things follow:

- **Names are matched as tokens, not as substrings.** `PDFBOX-4352-0.pdf` is a document of the
  ratcheted corpus and spells a corpus's name; a substring test read every sentence naming that
  file as a sentence naming that corpus. A path separator divides and a dot or hyphen inside a
  token does not, so `doc/corpora/pdfbox` offers `pdfbox` and the filename does not.
- **A cardinal denominates only above a hundred.** Below that, a cardinal in one of these
  sentences counts what the claim is about far more often than it names a population, and reading
  "the four corpus documents" as a denominator would silence exactly the claims the sweep is for.
  The cost is that a corpus smaller than a hundred documents cannot be named by its size; the
  report prints every population with its **name** beside its count, and the name always works.
- **And the report tallies which name answered each denominated claim**, so that a name that
  starts answering by accident shows up as a number that grew.

### The five rungs

1. **A fenced invocation** that walks some of this tree's corpora and not the rest. Not a sentence
   at all, and it reads first because two of the four findings took exactly that shape: a recipe
   is where a narrow population becomes a number somebody then writes down.
2. **An absence or a uniqueness in a ledger note**, naming no population — one witness anywhere
   refutes it, and a row's status rests on the sentence.
3. **The same beside the code**, where it says what a fixture is for; then **in a document**.
4. **A count**, naming no population. Refuting one takes a re-census rather than a witness.
5. **A numeral denominator no population on disk has** — a subset count and a population that
   moved underneath a sentence look identical from here, so both print and neither is judged.

A claim that names a population is counted rather than listed; so is one in `doc/adr/`, on
`parts`' rule that a decision record is dated by its own number and was right on its date.

## What it cannot see, said plainly

- **Whether a claim is true.** It reports an unstated denominator, never a wrong count.
- **A denominator in the sentence before.** §12.8.2.2's row is the standing example: its first
  sentence says "the corpus's one certification signature" and two sentences later the row names
  three populations. A person reads the paragraph; this reads the sentence.
- **An absence with no quantifier over a noun** — "the corpus holds none of them".
- **A claim counted in pages, glyphs or signature values** rather than in documents.
- **A recipe outside a Markdown fence.** ADR 0751's finding is a comment in a *Python* file, and
  this sweep reads Rust comments, the ledger and `doc/`. The first of the four is therefore the
  one it still cannot reach, which is worth knowing before trusting a clean run.

## Trap 13 — calibrated above a commit, against all four live defects

The pre-825 wordings of `doc/verify.md`, `crates/pdf-model/src/cms.rs` and
`doc/conformance/ledger.toml` were copied into the tree from `3c259925`, the sweep run, and the
tree restored. All four findings print:

| the defect, as it was written | where the sweep puts it |
|---|---|
| §12.8.5's row: "**No corpus document carries a document timestamp**, so the witness is a fixture" | rung 2, an absence in a ledger note |
| `cms.rs`: "the corpus's nine signed documents", three times | rung 3, beside the code |
| §12.8.3.4.2's row: "four corpus documents write" an indefinite length | rung 4, a count |
| `doc/verify.md`:528: "the eleven `/Contents` blobs the **nine signed corpus documents** hold" | rung 4, a count |

The fourth is the one that shaped the program twice over: it sits inside a fenced `sh` block, as a
`#` comment beside the command, and **both** Markdown readers in `tools/conformance` skip a fence —
correctly, because a shell line is not a sentence. It was in no population at all until
`fenced_prose` existed. And it is the sentence whose modifier defeated adjacency.

The corrected wordings beside them are **not** printed, which is the other half of the calibration:
§12.8.5's row as it stands names the crawl, and §12.8.3.4.2's names `doc/pdf.js`.

## What the first run found, and what this round paid

The run's own summary is the finding, and it is bigger than any sentence in it: of the claims that
quantify over a corpus, the large majority **name no population**, and of those that do, the two
names that answer almost all of them are `974` and `pdf.js`. *The corpus* is an unwritten
convention in this tree, and the convention is what four rounds have now been bitten by.

That is a backlog rather than a round, so this round paid what it could do properly and left the
rest to the command:

- **`doc/todo/01`'s two record sentences about §12.8.5** — 828's named debt. Both said the absence
  "holds"; both now say it holds of `doc/pdf.js` and of nothing wider, and the paragraph carrying
  the first is left standing as evidence that **a command without a stated population re-derives
  the same narrow answer**. That is the sharper lesson of the pair: the six-hundred-and-forty-first
  session's rule — a counted claim in a note owes a command — was necessary and not sufficient.
- **§12.8.3.3.2's row**, which said "there are three" after that same re-derivation. Re-run over
  every document this tree holds: **338 signature values in 325 documents** carry
  `adbe-revocationInfoArchival`, 335 of the values in the crawl and 3 in `doc/pdf.js` — the three
  the row names. So the material this row cannot check is *ordinary* rather than rare. Nothing the
  code does changes, because the note fires on the attribute and not on a population; what changes
  is that a round reading the row no longer takes three named files for the world.
- **The same sentence beside the code**, in `viewer-core`'s `notes.rs`, corrected with it. ADR
  0754's own lesson, applied: a claim in a row and the claim in the comment under it decay
  together.
- **§12.8.2.2's two sentences** naming "the corpus's one certification signature" now name
  `doc/pdf.js`. The row's later sentences already carried the crawl's 143 and its distribution;
  what was missing was the denominator on the sentence a reader meets first.

**No fixture is retired and no status moves**, which is ADR 0754's rule read the same way: a
witness found in a crawl ranks a format and cannot define one.

## Why it is not a gate

`parts`' argument unchanged. A sentence that has always meant the ratcheted corpus, written in a
round that had no other corpus to mean, is not a defect — and there are a thousand of them. What
the sweep offers is an ordering and a denominator to check against. Failing a build on it would
have it switched off within a round.
