# 0575 — A consensus of two, and what the missing third was doing

Status: accepted, in the seven-hundred-and-seventh session.
Supersedes nothing. Touches `crates/pdf-model/tests/oracle.rs` — one named list and one printed
line — and `doc/oracle-and-corpus.md`. **No verdict rule changed and no verdict moved.**

## The question

ADR 0542 made a missing reading visible and found six corpus pages judged on two references rather
than three. It did not ask the next question, which is a specification question rather than a
plumbing one:

> §6.3.2.2 and this project's own rule say a consensus is evidence about our reading of the
> standard. **Is a consensus of two the same evidence as a consensus of three?**

## The answer, in three registers, because the question has three

### 1. As an inference, it is the same kind and less of it

ADR 0005 is where the whole instrument rests, and its sentence is about a **pair**: two
implementations sharing no code arriving at the same picture is improbable unless the picture is
right. A third agreeing implementation multiplies that improbability; it is not what creates it. So
a consensus of two is evidence of exactly the same kind, weaker by one factor, and there is no
threshold anywhere in the argument at which it stops being evidence — which is why
`render_references` was written to tolerate one failure in the first place, and why its doc comment
was right.

### 2. As arithmetic, the two differ in opposite directions and the difference is not measurable here

`pdfref::decide` finds the largest **mutually** agreeing subset and widens the tolerance by the
spread of the pairs *inside* it. Two consequences, and they pull opposite ways:

- **the bound is tighter.** `Tolerance::widened_to` takes a maximum, so a bound derived from one
  pair is never looser than one derived from three pairs including it. That is trap 12's shape:
  where two references agree closely, the bound can be tighter than eight-bit arithmetic;
- **there are fewer comparisons to pass.** `we_match_all` is checked against every member of the
  consensus, so a trio asks three questions of us and a pair asks two.

Neither effect can be measured on these six, and the reason is the whole of the answer to the
question: **the third reading does not exist.** There is no counterfactual to compute — the
reference that is absent is absent because it cannot produce a picture of this document at all. A
sentence beginning "if `mupdf` had rendered it" is a sentence about a program that does not do
that.

What *can* be said, and is: none of the six is `contradicted`. Four agree and two are ambiguous, so
the population where a pair-consensus is doing the judging currently produces no accusation against
this tree.

### 3. As a precondition, the six are not about the count at all

ADR 0541's rule, which is ADR 0005's precondition rather than a new principle:

> a vote is evidence only where there is a clause the references are both reading

and `CLAUDE.md`'s: the standard "describes *valid* files and says nothing about the rest". That is
the ground on which `format-corpus` is excluded from the oracle's vote — three programs agreeing
about a broken file agree about three recovery heuristics.

**Five of the six lost their third reading because the document is outside what ISO 32000-2
describes**, and one because a reference is wrong. So what these pages raise is not *how many
references voted* but *why one of them could not*, and the second question has an answer per page
where the first has only a number.

## The six, one at a time

Each was reproduced by hand with `tools/pdfref/src/reference.rs`'s own invocation, because trap 3
binds a measurement taken outside the harness exactly as it binds one inside it.

| page | absent | what it met | whose |
|---|---|---|---|
| `GHOSTSCRIPT-698804-1-fuzzed.pdf` p1 | `mupdf` | its cross-reference subsection header is `00000004294967296 3` — an object number of 2³² (§7.5.4); `mutool` repairs, then reports `non-page object in page tree` and **0 pages** | the document's |
| `bug1606566.pdf` p1 | `ghostscript` | the file begins `%\xe2\xe3\xcf\xd3` — the binary comment line, with **no `%PDF–n.m` header** at all (§7.5.2); `gs` stops at file position 14 with `Error: /undefined in obj` | the document's |
| `bug_jpx.pdf` p1 | `poppler` | a JPX stream whose first box is not §7.4.9's JP2 signature box; `pdftoppm` falls back to raw J2K and **dumps core** on an OpenJPEG assertion | the document's *and* the reference's |
| `issue18986.pdf` p1 | `mupdf` | `cannot find page tree`, **0 pages**, on a file whose `1 0 obj` is a `/Pages` node reached from nothing | the document's |
| `issue21436.pdf` p1 | `mupdf` | `too many kids in page tree`, **0 pages** | the document's |
| `pr6531_2.pdf` p1 | `mupdf` | `cannot authenticate password` — and the empty password **is** this file's owner password | **the reference's** |

Three readings of that table, and each is worth more than the count of six.

**`pr6531_2.pdf` is a reference being wrong on a document we open correctly, settled by the
standard rather than by a vote.** Its encryption dictionary is `/V 5 /R 6 /CFM AESV3`, so
authentication is §7.6.4.4.11's Algorithm 12 over §7.6.4.3.4's Algorithm 2.B hash. Running that
algorithm on this file's own `/O`, `/U` and the empty password — independently of this tree, in
twenty lines of Python — gives **owner-auth true, user-auth false**; on `asdfasdf` it gives the
reverse. So the empty password is the *owner* password, §7.6.4.1 says authenticating that way
"should allow full (owner) access", and `poppler`, `ghostscript` and this tree open it while
`mupdf` 1.28 accepts only the user password. This tree has asserted exactly that since
`encryption.rs::an_empty_password_may_be_the_owner_password` was written; the computation above is
a second derivation of it that reads none of our code. **A reference that fails on a document we
open fine is evidence, and here it is evidence about the reference.**

**`bug_jpx.pdf` is the one where the reference's failure is not a refusal.** `poppler` does not
decline the file; OpenJPEG 2.5.4 aborts on `opj_int_ceildiv: Assertion 'b' failed` and the process
dies on a signal. A refusal is a reading — "this file is not something I can draw" — and a crash is
not. It is worth keeping separate for the same reason `HarnessError::RendererTimedOut` is its own
variant: a signal death is a property of the reference rather than of the document, even when a
document triggered it.

**And the other four are the same fact twice over.** In each, two renderers recover a page from a
file the standard does not describe and a third does not. Their agreement about that page is worth
having and it is not worth as much as an agreement about a valid one, because part of what they
agree on is *how to repair* — and no clause states that. Where such a page were ever to contradict
us, that is the first thing to establish.

## The decision

**Two references stay enough, and the count is not what to change.** No rule about how many
references form a consensus is altered: the inference is pairwise, the arithmetic's two effects
pull opposite ways and are unmeasurable here, and the population currently accuses this tree of
nothing.

What was changed is what a reader is told:

- **the reason** the third reading is missing, in the renderer's own words rather than the
  harness's — ADR 0574, which is what made this diagnosis possible at all;
- **a named list**, `JUDGED_WITHOUT_A_THIRD_READING` in `oracle.rs`, carrying the six with the
  reading above, and a summary line naming any page in that population that is **not** on it. `A
  count beside a list is not the list` (`doc/todo/02` §6), and a seventh page arriving is now
  legible immediately instead of being a number moving from 6 to 7.

**The list is printed and not asserted**, which is `doc/todo/05`'s standing rule kept rather than
waived: a figure enters a gate once it has held across rounds, and this population's membership is
a function of the machine's installed renderers as well as of the corpus. A round that sees it hold
twice may put a ratchet under it, and should say which of the two things it is ratcheting.

## What was considered and rejected

**Refusing an `agrees` verdict to a page judged on two.** It would demote four pages where nothing
is wrong, into a verdict that does not exist: `not comparable` means *fewer than two*, and
inventing a fifth degree of confidence would put every page nobody wants to explain in it — the
argument §2d already makes about `pdfref::Outcome` gaining a term the instrument cannot compute.

**Weighting a pair-consensus differently in the bound.** The bound is derived from the references'
own spread and nothing else, which is what stops it being a number somebody chose. A factor
conditioned on how many references drew would be exactly that number.

**Excluding a malformed document from the vote, the way `format-corpus` is excluded.** Tempting,
and wrong here: `format-corpus` is excluded as a *population*, decided once, because every file in
it is deliberately damaged and the exclusion is therefore a statement about the corpus. These six
are individual files inside a population of valid documents, and a rule that excluded a page
because a reference could not open it would let any renderer's weakness remove a page from
judgement. What the malformation earns is a sentence in the note, which it now has.
