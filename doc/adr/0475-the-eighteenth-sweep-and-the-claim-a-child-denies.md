# ADR 0475 — The eighteenth sweep, and the claim a child denies

Status: accepted.
Session: the six-hundred-and-forty-fifth, a sweep round under `doc/todo/01`'s binding rule.

## 1. What this decides

1. **`doc/todo/01`'s eighteenth sweep is `conformance --bin overstated`**, the thirteenth of them
   to become a program. Its subject is the failure shape the six-hundred-and-forty-first found and
   no committed sweep could print: a row claiming a *capability*, contradicted by its own children.
2. **Its discriminator is the ledger against itself** — a parent's assertion against a
   descendant's denial — rather than a row against the tree, and §5 says why that choice was
   available and what the other one would have cost.
3. **Two ledger rows are corrected by its first run**, §9.9.1 and §9.7.6, and one of the two is
   the fifth failure shape running the other way round: a parent that had *outgrown* its child.

### The two ordinals, because they had collided

This round was briefed as building "the thirteenth sweep", on `doc/todo/02` §4's count of **twelve
committed programs**, and that number is right — this is the thirteenth of those. But
`doc/todo/01`'s own header runs a *different* series, of sweeps rather than of programs, and in it
thirteen is taken: "twelve run every round, **a thirteenth run once and declined** (ADR 0265)",
with a sixteenth over the corpus (ADR 0405) and a seventeenth in `tools/spec-errata` (ADR 0426).
A second claimant to the same number was already in the file besides — the six-hundred-and-thirty-seventh
proposed "a thirteenth sweep somebody could write beside the twelfth", checking a note's prose
against its own `code` array, which is still unbuilt.

So the two counts had been running together, and one of them had two occupants. They are separated
here rather than added to: **eighteen sweeps, thirteen of them committed programs.** Both numbers
are derivable from `doc/todo/01`'s header, which now says so, and 637's proposal keeps its place in
the queue without owning an ordinal it never had.

## 2. The shape, and why the twelve committed programs are structurally blind to it

`doc/todo/01`'s fifth failure shape is a family head gone stale, and every recorded instance
before the six-hundred-and-forty-first had the parent **understating**: `partial` above children
that are all settled, a gap named that the subclause rows had closed. The sixth sweep is the
arithmetic for that direction, and `--bin counts` the arithmetic for the cardinals in its prose.

§12.11 was the other direction. Its row said "Table 276's handlers" among the things read, while
§12.11.1's said `/RH` "is unread" and §12.11.5's said "the `/RH` entry is read by nobody", and
nothing under `crates/` quoted `"RH"`. It was found by reading, not by any instrument, and the
reason is not an oversight:

| sweep | population | the term it hunts |
|---|---|---|
| seventh (`--bin inapplicable`) | rows claiming nothing is owed | a term the tree **names** |
| fourteenth (`--bin owed`) | rows claiming a debt | a term the tree **lacks** |
| eighteenth (`--bin overstated`) | rows claiming a **capability** | a denial in its own family |

An overstating parent names a thing the tree lacks — the seventh sweep's discriminator — under a
row claiming the opposite of a debt, which is the fourteenth's population inverted. **The sign is
reversed twice over**, so both existing instruments walk past it, and neither could be widened to
reach it without becoming the other.

## 3. The discriminator, and the three judgements a program can settle

A parent row saying an entry or a table **is read** makes a claim its own descendants are the
detail of, so both sides of the comparison are sentences in `ledger.toml`. No source is opened.
That is the whole instrument, and three things had to be decided inside it.

**The denial vocabulary is not new.** It is `unread::CLAIMS` unchanged — the words the second
sweep greps the tree with — so the two sweeps cannot come to disagree about what a denial is.

**The assertion vocabulary is five words matched on a word boundary**, and the boundary is what
lets it be five: "read" as a whole word is this ledger's verb for the tree consulting something,
and it is not reached by "unread", "reader", "reading" or "already". Against that stands one
idiom that looks like an assertion and is not — "**Read and kept** in the five-hundred-and-sixty-fifth",
which says a *round read the row* and claims nothing whatever about the tree. Two of the first
run's hits were that sentence and nothing else, which is why `NOT_ASSERTIONS` exists.

**Stance is a property of a clause rather than of a sentence**, and this sweep therefore cannot
use `unread::sentences`, which splits on a full stop for a reason its own doc gives. §14.12.4's
row is the witness: "Table 409's `/Start` and `/DParts` are read; Table 408 is not, and `partial`
is for that half" holds both stances inside one full stop, and read whole it asserts the exact
opposite of what it says. So the split takes the semicolon and the colon too. The cost is a
stance that carries across a colon into its own explanation, and that is the cheap direction — a
part dropped from the assertion list is a claim left for the next run, not a false hit.

### The three rungs

1. **The descendant denies the term itself.** Sharpest, and where the first run's live defect was.
2. **The descendant *owns* the term and denies reading** — its note opens by naming the term, it
   asserts nothing of it, and it denies reading something. **§12.11 is this rung and could be no
   other**: the parent named Table 276 and §12.11.5 denied `/RH`, the entry rather than the table,
   so no matcher joining term to term could have found it.
3. **The denial names another member of the same vocabulary** — another table where a table was
   asserted, another entry where an entry was. Printed rather than dropped, because what makes it
   noise is a judgement about which of two tables a sentence is about.

### The one noise shape a program can mark, and the attribution it needs

The dominant false positive is **a table read in part**: the parent names the entries it reads and
the child names the entries nobody reads, and both sentences cite the same `Table NNN`. §7.3.8's
`/Length`, `/Filter` and `/DecodeParms` against §7.3.8.1's `/FFilter` and `/FDecodeParms` is the
standing example, and both sentences are true.

Marking it needs the ninth sweep's rule rather than a plain key comparison, and the reason is
§12.11 itself. Its own row enumerates "Table 273's `/S`, `/V` and `/Penalty`, Table 275's
twenty-five types, Table 276's handlers" — so a mark that counted every key in the part as the
asserted table's would have demoted the Table 276 claim on the strength of Table 273's keys, and
**the one defect the sweep was built for would have printed as noise**. A key belongs to the table
the sentence attaches it to; `keys_attributed_to` is that rule, and a test holds it on §12.11's
own wording.

The second noise shape is left to the reader deliberately: **a capability read in part with no
table to divide it**. §14.9.2 says three of the four places a `/Lang` may occupy are read and
§14.9.2.2 says the fourth is read by nothing; both are true, both name `/Lang`, and only the
partitive tells them apart. A program deciding what "three of the four" governs is the
guess-what-the-sentence-means failure every sweep in `doc/todo/01` refuses.

## 4. The first run, and it found two live defects

Nine contradictions over 170 rows with descendants, asserting 118 terms between them, of which 49
are corroborated by a child. Four carry a mark that demotes them — two a table read in part, one
§12.11's own correction quoting its retired wording, one both — and two more sit on the third rung.
Of the three that remain, one is the partitive shape §14.9.2 and §14.9.2.2 make between them. **The
other two were defects.**

**§9.9.1 said Table 125's `/Length1`, `/Length2` and `/Length3` were "read by nobody", and §9.9's
own row had contradicted it for twenty sessions.** All three are read by
`pdf_font::program::stated_extent` since ADR 0459 — `/Length1` alone for a `/FontFile2`, because
Table 125 makes it "the entire TrueType font program", and the sum of the three for a `/FontFile`
— and the claim is checkable because each is stated in bytes "after it has been decoded using the
filters specified by the stream's Filter entry". What the lengths are *not* used for is the thing
the sentence was written about: `read-fonts` finds the eexec boundary in the bytes rather than at
`/Length1`, so no outline depends on them.

The row is the fifth failure shape with the sign reversed **inside one family**: it even carried
"**Read and kept in the five-hundred-and-forty-fifth session**", which was true when it was
written, and the six-hundred-and-twenty-fifth made it false without coming back. A parent that had
outgrown its child. The `partial` is unchanged and is the clause's one requirement on a processor,
which is still not executed: "If Length3 is 0, it indicates that the 512 zeros and cleartomark
have not been included in the FontFile font program and shall be added by the PDF processor."

**§9.7.6 said "Table 119's entries are read" and its own child says one of them is not.** Table
119 has six entries and `/BaseFont` is deliberately unread for a Type 0 font, on the clause's own
NOTE — "an arbitrary name, since there is no font program associated directly with a Type 0 font
dictionary" — which §9.7.6.1's row has said all along. Five of six, and the parent claimed the
table.

**The one instance the sweep was built from is confirmed by planting it back.** With §12.11's
pre-six-hundred-and-forty-first note restored, the sweep names it on rung 2, unmarked, quoting
both sides; with the corrected note it names the correction instead, marked as a row quoting the
wording it retired. That check is trap 13's, and it is the whole reason to run it: a sweep written
over the wrong side of a defect reports a clean tree.

## 5. The discriminator not taken, and what it would cost

The alternative was **a row against the tree**: a row naming a capability — a `/Key` it reads, a
table it consults, an algorithm it runs — that no source file names. It was not taken for three
reasons, and a later round can still take it knowing them.

- **Its answer side already exists and points the other way.** `--bin owed` measures exactly
  whether the tree names a term, over `partial` rows; the new sweep would be the same measurement
  over the rows that assert. That is a real instrument, but it is a second population inside an
  existing one rather than a new question, and `--bin quotations` is the precedent for how that
  should be built when it is built.
- **Its noise is the half a program cannot settle.** A row may legitimately describe what a
  *clause* requires rather than what this tree does, and the two are written in the same words. The
  chosen discriminator has no such ambiguity: both sides are this project's own claims about its
  own code, so a contradiction is a contradiction whatever the standard says.
- **It would not have found §12.11.** The parent's term was "Table 276", and source comments cite
  table numbers freely; a tree-facing matcher would have found a witness and reported the claim
  as corroborated. The contradiction was only ever visible from the child.

What it would cost to build: the term extraction and the reach count are `owed`'s and
`inapplicable`'s already, so the code is small; the price is entirely in reading, because the
population is every asserting row rather than the nine a family disagreement produces, and every
hit needs a judgement about whether the sentence is about the standard or about us.

## 6. Why it is not a gate

ADR 0249's ratio argument, and one of its own: **a parent row is allowed to summarise**, and the
difference between a summary that overstates and one that is true in part is a reading of two
English sentences. It runs in a fifth of a second over the ledger alone, and its output is read.
