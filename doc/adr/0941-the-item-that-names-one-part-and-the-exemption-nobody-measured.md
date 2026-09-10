# 0941 — The item that names one part, and the exemption nobody had measured

Session 944. Status: **accepted**. Continues ADR 0931, which established what a clarification is and
the four conditions a record has to meet; ADR 0933, which established that a resolution reaches the
parts it names and no others; and ADR 0935, which took A028 and deferred A010. This one closes the
last unclassified item of `TechNote 0010` and puts a number on the deferral.

## Part 1 — A014, the one item nobody had classified

`crates/pdf-archive/src/clarification.rs` carried A014 in a trailing sentence rather than in any of
its three lists: *a validator rule this crate has no row for at all*. That sentence was true and it
was not a classification, so the item was going to be rediscovered every time the note was read
again — which is the failure ADR 0933 recorded about A028 and ADR 0935 repeated the warning about.

### What the item is

Its title is about OpenType fonts in PDF/A-1 and its `Pertaining` line names ISO 19005-1 section
6.3.4 alone. The background is that the PDF 1.4 Reference's own font-descriptor table admits a
`/FontFile3` subtype named by *otherName* — a font program in some future format — so a literal
reading of ISO 19005-1 forbids nothing, while ISO 19005-1's clause 5.1 admits only what the PDF 1.4
Reference specifies, and that Reference specifies no OpenType.

Between the problem and the resolution the item carries a paragraph headed for validators of
ISO 19005-1, -2 and -3: such a validator shall fail an otherwise valid ISO 19005 document holding a
`Font` or `CIDFont` program whose `Subtype` entry the applicable PDF specification does not support.
That paragraph is why the item could not simply be filed under `doc/questions/A17`'s rule that part
1 is not a target — it is addressed, in as many words, to a validator of the part this crate does
target.

### Why it binds ISO 19005-1 and nothing else

Two arguments, and they are independent, which is the reason both are recorded.

**The first is ADR 0931's third condition, read against the note's own shape.** This crate already
acts on the *ISO 19005-1, -2 and -3 validators* paragraph of three items — A012, A020 and A029 —
and it does so because of a sentence those three have and A014 does not. Each of the three carries
a heading *PDF Validation TWG proposal*, and each one's `ISO WG Resolution` adopts it explicitly:
the three parts should be read as if the proposal above were part of the specification. The
validator paragraph is the operative detail of the proposal the working group adopted, which is how
A020's list of basic types and A029's empty-array rule reached this table.

A014 carries no proposal heading at all. Its resolution adopts nothing and states one thing: that
although the PDF 1.4 Reference permits OpenType as a `Subtype` value, the OpenType format is not
recognised there, and an ISO 19005-1 validation error follows. One part, one base specification.
Condition 3 says the case above a resolution is not the resolution, and here the working group's
verdict names part 1 twice over — in the `Pertaining` line and in the resolution's own sentence.

**The second does not depend on how the note is laid out**, and it is the stronger of the two
because it survives a reader who disagrees with the first. The validator paragraph states its rule
against *the applicable PDF specification*. For ISO 19005-2 that specification is ISO 32000-1, and
for ISO 19005-4 it is ISO 32000-2 — and both define the same three subtypes. ISO 32000-2 §9.9.1:

> The name shall be Type1C for Type 1 compact fonts, CIDFontType0C for Type 0 compact CIDFonts, or
> OpenType for OpenType fonts.

ISO 32000-1:2008 §9.9 states it in the same words, and both go on to specify the OpenType
`/FontFile3` row in full. So the ambiguity A014 exists to resolve — a `Subtype` the base
specification names but does not define — is one only the PDF 1.4 Reference has. Applied to part 2
or part 4 the paragraph would resolve a question those parts do not raise.

### What was done, which is nothing to the table

A014 is recorded in `clarification.rs` under a heading of its own, **reaching no part this crate
targets**, with both arguments and with the residue named. No row was added, and the decision not to
add one is the substantive half of this section.

The residue is real: a document stating a `/FontFile3` whose `/Subtype` names none of the three the
base standard defines goes unreported here. That is a requirement of ISO 32000, bound on a
conforming file by each part's clause 5.1 — and this crate already has a row for exactly that,
`conformance/adheres-to-the-base-standard`, whose `Unchecked` reason is an argued decision: this
table checks the requirements ISO 19005 *adds*, and whether a document conforms to the whole of its
base standard is a different and far larger question that this project tracks about its own reading
rather than about a document.

Two section 5.1 rows do exist — the file identifier and what a hexadecimal digit is — and it is
worth saying why they are not a precedent for a third. Both are cases where an ISO 19005 clause
states a rule whose *terms* the base standard defines and the part does not: section 6.1.6 states a
digit count and never says what a digit is. A `/FontFile3` subtype is not that shape. No clause of
either part is about it, so adding a row would be this table quietly taking on a piece of ISO 32000
conformance it has already decided, in writing, not to take on — and taking it on because a
technical note about a different part happened to mention it.

## Part 2 — the exemption, measured

ADR 0935 deferred A010 and gave the argument in two halves: what is missing from this crate is not
A010 but section 6.2.2's *published* exemption, which A010 only narrows; and implementing that
exemption is a whole-file reachability problem where an error in the permissive direction withdraws
real failures in silence. Both halves stand. What the ADR could not say was how much was being
deferred, and it said so: a claim about the corpus rather than about the world, and no witness.

### The instrument

`crates/pdf-archive/examples/unreferenced.rs` computes, per document, the objects reachable **only**
through a named resource entry the associated content stream does not reference, and then asks which
of the report's failures land on nothing else. It is a command rather than a number in a document,
which is `CLAUDE.md`'s rule about derived facts and is also the practical point: the answer moves
whenever a row does.

It is not the implementation and is not on the way to being one. It is a whole second pass over the
document built for measuring, where the real thing has to answer the same question from
`Examination` without walking the file again. What it does share with the real thing is the reading:
a resources dictionary is associated with the stream that states it, the page whose `/Contents` it
governs, or the glyph procedures of the Type 3 font that states it — A003's sentence — and a
dictionary with no such owner, an `/AcroForm` `/DR` being the standing case, exempts nothing at all,
because the clause's premise is an associated content stream and there is none.

One approximation, stated in the file: every name operand of a content stream counts as a reference,
with no operator table. It can only make the exempt set smaller, so the output is a floor.

### What it found

Measured over all 1 552 documents of `doc/veraPDF-corpus` under all six targets, on the day this
was written:

- 169 state a named resource their associated content stream does not reference, and 147 hold an
  object reachable only through one.
- **12 fail a row whose every finding is on such an object** — 5 under `PDF_A-4`, 7 under
  `PDF_A-2b`, none under the other four targets.
- **All 12 fail nothing else.** The verdict itself turns on the exemption in every case, which is
  the strong form of the question rather than the weak one.

The figures are here because an ADR is a dated record; the live number is the command's, and
`doc/todo/62` is where a later round is sent for it.

The finding that matters is what clauses they fail under. All five part 4 witnesses fail under
section 6.1.6.1 or section 6.1.7 — `6-1-6-1-t02-fail-a`, `-b`, `-c`, `6-2-2-t04-fail-a` and
`-fail-b` — and part 4's exemption is published with sections 6.1.6 to 6.1.9 carved back out. All
seven part 2 witnesses fail under section 6.1.7.1 or section 6.1.13 — `6-1-7-1-t01-fail-b`,
`-t02-fail-a`, `-t03-fail-a`, `-t04-fail-a`, `-b`, `-c` and `6-1-13-t04-fail-a` — and A010 carves
out sections 6.1.2 to 6.1.13. **Not one document in the corpus would change verdict under the
exemption as it actually stands in either part.**

### What that changes about the work

ADR 0935 ordered the two pieces: the exemption first, A010 after it, on the reasoning that taking
A010 alone would be implementing nothing. That ordering is wrong, and the measurement is what shows
it. Taking part 2's published sentence without A010 would withdraw the seven part 2 agreements
above — each one a `-fail-` document whose author put the fault in an unreferenced resource
deliberately. **A010 is not a refinement to apply afterwards; it is what makes part 2's
exemption safe, and the two have to land together or not at all.**

That the corpus was built this way is itself evidence worth naming carefully. It is evidence about
the veraPDF consortium's reading — they wrote both the corpus and the technical note — and under
`CLAUDE.md` principle 5 it is not the ground of anything. The ground is A010's own resolution, which
this tree holds and has read. What the corpus adds is the cost of getting the order wrong, which is
the thing a specification cannot tell anybody.

The direction of risk is also now stated rather than assumed, and it is one-sided. `over` cannot
move: an exemption only ever withdraws failures, and `over` is zero on all six targets. Every unit
of risk is in `missed`. ADR 0935 called A010 the item that could move `over` off zero in either
direction; that was true of a *wrong* implementation and not of this rule, and the distinction is
worth keeping because it decides what a round taking this work has to guard.

## What it would take to change this

For A014: an erratum, or a later technical note that resolves the validator paragraph for parts 2
and 3 in its own resolution — or, failing either, a base specification that admits a `/FontFile3`
subtype it does not define, which would give the paragraph something to resolve in a part this crate
targets. For the residue: a decision to check ISO 32000 conformance here, which is
`conformance/adheres-to-the-base-standard`'s to revisit rather than a font row's. For the deferral:
`doc/todo/62`, whose first requirement is a reachability answer on `Examination` and whose numbers
are a command away.
