# 0935 — The clarification that adds failures, and the one that withdraws them

Session 943. Status: **accepted**. Continues ADR 0931, which established what a clarification is,
and ADR 0933, which established that a resolution reaches the parts it names and no others. This
one is about the last two items of `TechNote 0010` that were owed: A028, taken, and A010, deferred
with an argument rather than left on a list.

## A028, and what it actually resolves

The item is titled for inheritance of default colour spaces, and its *Pertaining* line names
ISO 19005-2 section 6.2.2 and ISO 19005-3 section 6.2.2. Its background is that both parts require
a content stream referencing other objects to have an explicitly associated resources dictionary,
and say nothing about whether §8.6.5.6's `/DefaultGray`, `/DefaultRGB` and `/DefaultCMYK` are among
the objects that sentence reaches.

It carries an **ISO WG Resolution**, and not merely a problem statement or a TWG proposal — ADR
0931's third condition, which four items of this note fail. The resolution is that parts 2 and 3
are read as if the proposal above it were part of the specification, and the proposal has two
sentences: any default colour space of §8.6.5.6 shall also be defined in the explicitly associated
resources dictionary, and a processor shall ignore any resource that dictionary does not define.
Its own NOTE names what that prohibits: inheriting an omitted `Resources` entry in a form
`XObject` or a Type 3 font from the page the thing is drawn on.

Read beside A003 — already carried here, which fixes "explicitly associated" as the `Resources`
entry of a page, a tiling pattern, a form `XObject` including an annotation appearance, or a
Type 3 font dictionary — the consequence is sharp. **A content stream that states no `Resources`
entry of its own has no explicitly associated dictionary at all, and therefore reads no default
colour space whatever.** Not the page's; not the invoking stream's.

## What that costs a file, which is the reason this round had the corpus in front of it

`crate::survey` read §8.6.5.6's three defaults from the resources *in force*, which for a form
`XObject` with no `Resources` entry is the page's. So a `0 g` inside such a form was licensed here
by a `DefaultGray` the working group says to ignore, and taking A028 withdraws that licence. That
is the `over` direction — this crate failing a document its author built to conform — and it is
the direction session 942 flagged as needing a round with the corpus open rather than a round with
the note open.

**The corpus cannot rank it.** Measured before the rows changed, over all 1 552 documents of
`doc/veraPDF-corpus` under all six targets:

- 41 documents have a device colour whose licence turns on a default colour space in force.
- 7 documents run a content stream against a resources dictionary it fell back on.
- **The two sets are disjoint.** Not one document is in both, so no `DeviceColour` and no
  `GroupSpace` anywhere in the corpus has a default that A028 removes.

The seven are `6-2-2-t02-fail-a`, `-b`, `-c` and `-t03-fail-a` under `PDF_A-4`, and
`6-2-2-t04-fail-d`, `-e` and `-f` under `PDF_A-2b`. Every one of them is a `-fail-` document that
this crate already fails under
`graphics/content-streams-have-an-explicit-resources-dictionary`, so even the near miss is a file
whose author built it to break the very clause A028 clarifies.

The sweep after the change is byte-for-byte what it was before it, timing line aside: `over` stays
zero on all six targets, `agreed` and `missed` are unmoved, and no witness changed side in either
direction. That is the second time a clarification has widened a rule with the corpus unable to
see it — A002 was the first, and ADR 0933 drew the same conclusion for the same reason.
`CLAUDE.md`'s two denominators in miniature: the coverage question moved and the robustness
question could not rank it, so the reach is pinned by a test rather than left to the sweep.

## Where the reading lives, and why the survey now records two of them

A028 names ISO 19005-2 and ISO 19005-3. It does not name ISO 19005-4, which was published nine
years after the note and states its own section 6.2.2 in its own words — the explicit-dictionary
sentence, the define-every-name sentence, and the unreferenced-resource exemption — with nothing at
all about default colour spaces. ADR 0933's rule therefore binds: printing this resolution's reach
wider than the working group drew it is inference, not reading.

So the survey records both readings rather than one, in a new
`crate::survey::DefaultSpace`: `in_force` is what the resources in force name, which is what part
4's rows read and what a *renderer* obeys; `explicit` is the same value where those resources are
the stream's own explicitly associated dictionary, and it is `None` where they are not. Part 2's
`licensed_by_default_under_part_two` reads the second, part 4's
`licensed_by_default_under_part_four` reads the first, and the difference between the two fields is
exactly the clarification.

Six rows of `crate::table::graphics` bind part 2 and turn on a default, and every one of them now
carries the record so a verdict says which reading it applied: the three device-family rows of
section 6.2.4.3, section 6.2.4.4's alternate spaces, section 6.2.4.5's underlying spaces, and
section 6.2.10's transparency group `CS`. The `DeviceGray` row cites `A026 and A028` together —
two different questions that meet at one sentence, which is the second use of the two-item form
that `A012 and A023` established.

**The rendering side is untouched and must stay untouched.** `pdf-model` implements §8.6.5.6 as
published, inheritance and all, because that clause is what a PDF processor obeys when the file is
not being validated against ISO 19005-2. A028 changes what a *validator* asks of a PDF/A-2 file; it
does not change what a page looks like. §8.6.5.6's ledger row now says so, because the next round
to read that row beside this crate would otherwise have to guess.

## A010, deferred, and the argument for deferring it

A010's resolution replaces section 6.2.2's exemption sentence — a named resource no content stream
references is exempt from all requirements of the part — with one that keeps sections 6.1.2 to
6.1.13, and adds that such a resource shall still conform to ISO 32000-1. It is genuinely
resolved, by the working group, naming parts 1, 2 and 3.

**It is deferred, and the first half of the argument is that A010 is not the work.** What is
missing from this crate is section 6.2.2's *published* exemption, which no row states at all. A010
only makes that exemption narrower. So taking A010 without first implementing the sentence it
modifies would be implementing nothing; and implementing the sentence is a new requirement rather
than a reading of one.

The second half is why that requirement is not a session's worth of plumbing. Half of it already
holds by construction: the rows that read `crate::survey` never see a resource nothing references,
because the walk reaches a form `XObject` only through the `Do` that names it. The other half is
the object walk — `Examination::objects` enumerates every object a cross-reference section names,
and the font, filter and image rows judge an unreferenced one like any other. Exempting it means
deciding, of an object, that **every** path to it is as an unreferenced named resource. That is a
whole-file reachability question:

- A font object can be an unreferenced `/Font` entry on page 1 and a referenced one on page 2. The
  exemption is stated per content stream; the row is stated per object.
- The same object can be reached by a route that is no resources dictionary at all — an
  `/AcroForm` `/DR`, an annotation appearance's own resources, a `/CharProcs` entry.
- The carve-out is by clause, so the filter has to know which sections 6.1.2 to 6.1.13 are, which
  makes it a change to `Examination`'s contract rather than to any one row.

Get that wrong in the permissive direction and the crate **withdraws real failures in silence** —
`missed` rises, and a validator that has quietly stopped checking something looks exactly like one
that never checked it. That is the one direction a validator may not move by accident, and it is a
worse failure than the over-report it would be fixing. Its own round, with the corpus in front of
it and its own ADR, is the right shape; a round already spending its evidence on A028's `over`
column is not.

The honest statement of the cost of waiting: this crate over-reports today against section 6.2.2's
published text, and no corpus witness demonstrates it. That is a claim about the corpus rather
than about the world, and it is exactly the kind of claim `CLAUDE.md` says a corpus cannot settle.

## What is left of `TechNote 0010`

Nothing owed but A010. Every other item of the note is taken with a check changed, taken as a
confirmation, or already true with nothing to cite — except A014, which is a validator rule this
crate has no row for at all: a font or `CIDFont` program whose `Subtype` the applicable PDF
specification does not define. `crates/pdf-archive/src/clarification.rs` carries the whole list
under those headings, and it is checked against the note rather than against its predecessor —
session 941's copy omitted A028 outright and called A002 already true when the row it named bound
the wrong part, and both were found only by reading the note again.

## What it would take to change this

For A028's reading: an erratum to ISO 19005-2, or a later technical note that revisits it. For the
two-field `DefaultSpace`: an item of some future note that names ISO 19005-4, at which point the
two fields collapse back into one. For A010's deferral: a round that implements section 6.2.2's
exemption, at which point A010 is a line of it rather than a task of its own.
