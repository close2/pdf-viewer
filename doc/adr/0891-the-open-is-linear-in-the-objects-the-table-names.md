# 0891 — The open is linear in the objects the table names, and no laziness makes it otherwise

Session 925. Status: **accepted**. The second of this round's two records;
[ADR 0890](0890-where-a-large-documents-open-goes-and-the-page-tree-walk-on-the-launch-path.md)
is where the numbers come from.

## Context

`CLAUDE.md` principle 2: "**A 500-page document must open no slower than a 5-page one.**"
[ADR 0885](0885-what-the-launch-path-costs-and-which-of-principle-2s-claims-hold.md) measured it
false by about thirty times and left the wording to the owner. The instruction to this round was
to find what scales and then decide honestly whether it can be made lazy or whether the sentence
is wrong. This is the second half of that.

## The three things that scale, and what each is a function of

| | is linear in | 5 pages | 1023 pages |
|---|---|---|---|
| `Document::open` | the entries the cross-reference sections state | 247 | 112 269 |
| `Pages::indices` | the pages the tree holds | 5 | 1023 |
| `Outline::read` | §12.3.3's items | 14 | 988 |

None of the three is linear in the *page count*, which is what the sentence names; two of them
correlate with it and one does not correlate with it at all. A one-page document with 60 000
objects opens like a book.

## What could be made lazy, and what it would buy

Two of the three are there for one reason: the window caption's section name, which
`Viewer::announce_page` asks for the moment a document opens. Defer that and the 1023-page
document's open falls from about 11.3 ms to about 4.4 — **and the ratio to the five-page
document's open falls from 49× to 19×.** The sentence is still false by nineteen times.

So the deferral does not save the sentence, and it costs something real: the caption would show
`Page 1 of 1023` and gain `— Foreword` only after the first page turn, or after a panel asked for
the outline, or on a tick — a second `Event::PageChanged` every host would have to be written to
expect. **A change that is host-visible, that principle 2's own "deferred until first use" asks
for, and that does not make the sentence true, is a change for the owner to ask for rather than
one for a measuring round to make.** It is in `doc/questions/Q25` with this arithmetic beside it.

## What cannot be made lazy, and why the argument is not "it is hard"

**A cross-reference table is a table, and §7.5.6 makes reading all of it the price of reading any
of it.** The clause's precedence rule is that the most recent section wins, so an entry's meaning
is a function of every section from the newest back through the `/Prev` chain — a reader that
decoded one section lazily would have to decode all of them before it could answer for one object
anyway. ISO 32000-2 states 112 269 entries across two sections in 39 KiB of `FlateDecode`d,
PNG-predicted data; inflating that, un-filtering it and turning it into locations is 4.18 ms, or
37 ns an entry, and about a third of what opening the document costs.

What *could* be deferred is the shape rather than the reading: keeping the decoded records as
bytes and indexing into them on demand would drop the sort and the `BTreeMap` build (about 4.3 M
of the open's instructions) while keeping the inflate and the predictor. That is a fifth of a
third — worth having one day, worth nothing to this sentence.

**And the truncating alternative was priced and refused.** `section_at` only cares about
destinations on pages `0..=index`, so the walk could stop at `index` — which at open is page one.
It is a regression: `Destination::resolve` answers a target the map does not hold by
`document.get(id)`, which parses *and clones* the object, and the outline names about as many
distinct pages as the document has. Trading one cheap key read per page for one dictionary clone
per outline item is the wrong direction, and it is written down here so that the next round does
not re-derive it (`doc/habits.md`'s *Measuring*: a price is a claim).

## The other false sentence, and what session 920 did to it

ADR 0885's other finding was that "**No system font enumeration**" is false without a condition: a
page naming a font it does not embed walks the machine's font directories on the launch path and
roughly doubles time to first page. Session 920's resource port (ADR 0880) is often read as having
changed that, and it did not:

- **It changed who enumerates, not whether.** A confined worker asks by *description* and the
  broker — the host process — matches, opens and answers. The directory walk still happens, still
  on the launch path of the first page that needs a substitute, and now with the face's bytes
  copied across a pipe as well.
- **It changed the default in the other posture.** A host that calls neither `Host::offer` nor
  `faces_come_from` gets a worker with no machine fonts at all (session 914), so *there* the
  sentence is true — and the page draws with what the file gave it.

So the port makes the sentence *conditional on the host* on top of being conditional on the
document, which is a reason to reword it rather than to keep it. Both sentences want the same
qualifier and principle 2 already contains it two bullets down — "[a]nything **not needed to show
page one** is deferred until first use". A substitute font is needed to show page one. So is the
caption's section, if the caption is part of showing it, and that is the question.

## Consequences

- Nothing in `CLAUDE.md` was amended by this round, deliberately, for the third round running.
- `doc/questions/Q25` gains a fourth and fifth item: the open's sentence with the arithmetic that
  says no deferral rescues it, and the font sentence with what the port did to it.
- The claim "reading the cross-reference table is linear in the objects it names" is a claim about
  §7.5.6 and decays like any other. It is written with the clause beside it so that a round that
  thinks otherwise knows which sentence to argue with.
