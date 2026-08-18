# ADR 0394 — A caret, the node that may carry it, and what a page turn was paying for

Status: accepted, 2026-08-17. Session 559. Takes two of `doc/todo/31`'s remaining entries — AT-SPI's
`Text` interface, and what the question costs on a thousand-page document — and leaves the third,
actions, with a sharper reason than it had. Amends §14.7's, §14.7.5.1.1's, §14.8.2.5.1's and
§14.9.3's ledger rows. Extends ADR 0214's bridge; changes nothing ADRs 0301, 0325, 0338 or 0342
decided.

## The two questions, and why they were taken together

`doc/todo/31` ranked the `Text` interface first and the cost third, and they turned out to be the
same round's work for a reason neither entry stated: the interface is built out of the page's text
layer, per character, so anything it costs is added to a query the file already suspected of being
too slow. Measuring the cost first was the only way to know whether the interface could be afforded
at all.

## What a screen reader was owed, and what it had

§14.7 exists so that a document can state the order its content is *read* in, and §14.8.2.5.1
defines that order as "a depth-first traversal of the document's logical structure hierarch y".
Since ADR 0214 this program has put that order on AT-SPI as a tree of named nodes, so an assistive
technology can hear a paragraph. What it could not do is **move through** one — by character, by
word, by line — or say where a caret is, which is `org.a11y.atspi.Text` and which is most of what
reading a document with a screen reader actually consists of.

Nothing was missing from the data. `Interpretation::text_layer` has been one `Placed` per character
code since ADR 0118, each with the range of the readback it produced and the quadrilateral it
occupies. What was missing was the shape.

## Decision 1 — the answer carries lines of characters, and the host builds the platform's arrays

`viewer_core::AccessibilityNode::lines` is a `Vec<TextLine>`, one entry per line of the element's
**own** text — the same "own" [`AccessibilityNode::name`] already means, because a container that
repeated its children's characters would be walked twice. A `TextLine` is a string and one
`Character` per code, each stating how many bytes of that string it produced and where it is in the
viewport's device pixels.

Three things about that shape are decisions rather than transcription:

- **The unit is the character *code*, not the Unicode scalar.** A code mapped through `/ToUnicode`
  to `ffi` drew one glyph in one place; splitting its box into thirds would invent positions the
  file does not state. AccessKit's own definition agrees — "[a] character is defined as the smallest
  unit of text that can be selected. This isn't necessarily a single Unicode scalar value" — and
  says outright that it cannot compute the lengths itself.
- **A line is where the glyphs landed.** A PDF states no line breaks: §9.4.2's `Td`, `TD` and `T*`
  move a pen and §9.4.3's `TJ` adjusts it, so a reader recovers a line or invents one. The
  recovery is `select::continues`, below.
- **It is the readback and not the speech.** `name` applies §14.9's substitutions; this does not. A
  caret moves over what is on the page and `GetCharacterExtents` asks where a *glyph* is, so a run
  built out of an `/Alt` would report positions for characters nobody drew. An element stating
  §14.9.3's `/Alt` or §14.9.5's `/E` therefore carries no lines at all, which is the same stop
  `substituted` already puts on the walk.

The **platform's** arrays are the host's: `viewer-accessibility` turns each line into character
lengths, positions and widths in AccessKit's own terms, because those are stated "in the direction
given by `text_direction`" and the direction is a fact about the platform's axes rather than about
the page.

## Decision 2 — a line's own join predicate, because a highlight and a caret want different ones

`select::quads_for` has merged glyphs into runs since selection existed, and reusing its `joins`
looked free. It is not, and the measurement is the argument: under `joins`, ISO 32000-2's cover
answers `In`, `terna`, `tiona`, `l `, `Sta`, `nda`, `rd ` where the page says *International
Standard*. Its display face is tracked, so glyph boxes overlap their neighbours slightly, and
`joins` ends a run at the first overlap greater than a hundredth of a unit.

A highlight does not care: the two rectangles abut and a person sees one band. A caret does: those
are seven lines to move through where a person sees one. So `select::continues` states the same
question with tolerances scaled to the glyph's own height — the same baseline within a twentieth of
it, no gap wider than one and no overlap deeper than half. On the whole corpus that is the
difference between **1 004 514 lines and 114 010**, for the same 2 974 184 characters.

`joins` is deliberately left alone. A selection's merge is a statement about what a person dragged
across; changing it to suit a caret would move a feature nobody was measuring.

## Decision 3 — the page node takes `Role::Document`, and that is AccessKit's decision

This is the part where the transport cannot carry what the clause states, and it is recorded as a
choice rather than approximated (the precedent is ADR 0338's signature field).
`accesskit_consumer::Node::supports_text_ranges` is:

```text
(self.is_text_input() || matches!(self.role(), Role::Label | Role::Document | Role::Terminal))
    && self.text_runs().next().is_some()
```

so **no role any of §14.8.4's forty-one types maps to can carry the interface**. Not `Paragraph`,
not `Heading`, not `Cell`. The two ways out were:

- **map `P` to `Label`** — trade the vocabulary §14.8.4 spends five tables defining for a platform
  interface. Refused: it is exactly the trade ADR 0338 refused when it declined to call a signature
  field a button, and it would tell a person that a document has no paragraphs.
- **put the interface on the page**, which is *this program's own* node rather than a structure
  element's. It stood between the document and its elements as an unnamed `Group`, and AT-SPI's
  `DocumentFrame` is what a text-bearing document container is there. Taken.

Every element keeps the role §14.8.4 gives it, and because the runs are published in the answer's
own order the caret crosses the page in §14.8.2.5's order — which is the whole reason a tagged
document is worth reading.

**The runs are invisible.** `accesskit_consumer::common_filter`, which every platform adapter
applies, answers `ExcludeNode` for a `Role::TextRun`, so not one of them appears on the bus: the
tree an assistive technology walks is the tree it walked before, with an interface on it that was
not there.

**What this does not do**: give each *element* its own text interface. A client asking a paragraph
for `org.a11y.atspi.Text` still gets nothing. That is the platform's answer rather than this one's,
and `doc/todo/31` records it upstream beside `Table`, `TableCell` and the relation set — one
question for all four.

## Decision 4 — a long line is several runs that say they are one line

AccessKit holds word starts as indices into a run's characters in a `u8`, so a run of more than 255
characters could not state where its later words begin. A longer line is published as several runs
joined by `previous_on_line` and `next_on_line`, which is the pair `is_line_start` and `is_line_end`
read: the line stays one line to a caret moving by line, and the arrays stay addressable. It is not
hypothetical — a full-width table row of ISO 32000-2 exceeds it.

**Word boundaries are a choice and are marked as one.** AccessKit declines to compute them and says
why: "users will get unpredictable results if the word boundaries exposed by the accessibility tree
don't match the editor's behavior". There is no editor here and no clause either — §9.4 has glyphs
and positions and no word — so the rule is the plainest available: a word begins where a run of
whitespace ends.

**And the space between two words usually has no glyph.** §9.4.3's `TJ` moves the pen instead of
drawing one, so the readback holds a space that no `Placed` entry accounts for. It is attached to
the character *before* it, which is AccessKit's own convention — "[t]railing whitespace is typically
considered part of the word that precedes it" — and without it a run would say `twowords`.

**Whitespace only**, which is a guard rather than a tidy-up: a line is decided by where the glyphs
landed, so two glyphs beside each other on the screen may be far apart in the readback with another
element's words in between, and those are not this element's to speak. Adding the condition moved
no count on the corpus, which is the answer that makes it cheap to keep.

## What the cost turned out to be, and what it is now

`viewer-core --example accessibility_cost` on `doc/ISO_32000-2_sponsored_EC3.pdf`, 1023 tagged
pages, page 700. **A stopwatch is the wrong instrument on a busy machine** (ADR 0312), so the A/B is
`valgrind --tool=callgrind --collect-atstart=no "--toggle-collect=*Viewer*::query*"`, which counts
the query and nothing else; a *warm* page turn is the difference between three repeats and one,
over two.

| instructions | before | `Tree::child` alone | with the caret as well |
|---|---|---|---|
| cold, first query on the page | 264 451 233 | 239 386 443 | 241 213 800 |
| warm, what a page turn costs | **65 861 010** | 41 942 513 | **43 796 275** |

**70.8% of the query was `Tree::identified_children`**, and inside it `Tree::child`, which reads one
`/K` entry. It asked `Document::resolve` **twice** — once to test for §14.7.5.1.1's bare integer and
once for the dictionary — and then cloned the dictionary a third time into `Child::Element`. Each of
those is a deep copy of a structure element including its own `/K` array. It now resolves once and
moves.

Why that is where the time was, and it is a fact about the *shape* of the walk rather than about
this document: ADR 0325 made the walk descend only into the subtree the page occupies, but deciding
whether a child is in that subtree still means resolving it, because §14.7.5.1.1's content items may
themselves be indirect objects. So a page near the end of a thousand-page document pays for every
child of every ancestor between it and the root, and the ancestors near the root have thousands.

**The skip ADR 0325 rejected is still rejected**, and the reason is unchanged: a child skipped on its
reference alone could be an indirect content item, and skipping it would take an annotation off the
page silently. A variant was considered this round and also rejected — skip unresolved only under an
element that §14.7.5.4's parent tree does *not* name as an owner — because it would lose exactly the
case `doc/todo/31`'s second residue is about, a `/StructParents` array shorter than the page's
sequences, which the present walk partly rescues.

**The caret costs 1.85 M instructions warm, 4.4% of the query**, which is what makes it affordable
at all. Net, a page turn on the largest document this project holds went from 65.9 M to 43.8 M
instructions, **a third less**, while gaining an interface it did not have. Wall clock over eleven
repeats moved from 46.3 ms to 11.7 ms on the same machine, which is a larger ratio than the
instruction count and is exactly why the instruction count is the number quoted: the earlier figure
was taken while nine other builds were running.

## How it was verified, and it is the bus

`doc/verify.md`'s recipe, unchanged: `dbus-run-session`, `at-spi-bus-launcher`, `at-spi2-registryd`
with a `DISPLAY` of its own, `Xvfb`, `IsEnabled` set on the session bus, and a client walking
`org.a11y.atspi.Accessible` from the registry root — this time asking `GetInterfaces` at every node
and, where `org.a11y.atspi.Text` is among them, `CharacterCount`, `GetText`, `GetStringAtOffset` by
word and by line, and `GetCharacterExtents`.

`doc/PDF20_AN001-BPC.pdf` page one, which is ADR 0214's own witness:

```text
[DocumentFrame] "page Cover (1 of 5)"  +Text
   CharacterCount=165
   GetText(0,count)="A ppl ication NotePDF 2 .0 A pplication Note 0 01: B lack  Point …"
   word at offset 0=("A ppl ication ", 0, 14)
   line at offset 0=("A ppl ication Note", 0, 18)
   extents of character 0=(282, 236, 14, 32)
  [Image] "PDF Association logo"
  [Paragraph] "A ppl ication Note"
  [Paragraph] "\nPDF 2 .0 A pplication Note 0 01: \nB lack  Point  Compensation"
```

The page carries the interface and the `Paragraph` under it does not, which is the platform
constraint above, visible from a client. The extents are screen pixels, which is what a magnifier
tracking a caret needs. `structure_simple.pdf` is the small witness: 52 characters on the page node,
lines `Heading 1`, `This paragraph 1.`, `Heading 2`, `This paragraph 2.` — the document's four
elements in §14.8.2.5's order.

**One thing the bus showed that the code did not.** `Role::Label` *is* in AccessKit's admitted set,
so the `Span` and `Lbl` elements that already map to it carry the interface too, at no cost and
without anything being asked of them. That is a windfall rather than a design, and it is recorded
because the next round should not read it as one.

## What the census says

`tools/state.sh accessibility`, before and after, over the same 988 documents. Every existing count
is unchanged — which is the point, since none of this was meant to move them — and one line is new:
**57 115 of the 102 849 elements reached now have somewhere a caret can stand**, on 114 010 lines of
2 974 184 characters. Beside it is a class rather than a count, and it is empty: a line whose
characters and text disagree, which no consumer could index. `TextLine`'s invariant is that the sum
of the characters' bytes is the string's length, every platform indexes one by the other, and a
breach of it would show up as a screen reader reading the wrong word rather than as a crash. The
census asserts it over the whole population.

## What is left on `doc/todo/31`

**Actions.** Unchanged in substance and sharper in shape after this round: the tree declares none,
so a conforming client requests none, and one that arrives anyway reaches `Bridge::requested` and is
printed by name. (Taken in the five-hundred-and-ninetieth session, and the two invitations this
paragraph names are what it declared. ADR 0425.) This round adds a second invitation to the one ADR 0338 added — a page that says a
caret may move through it invites `ScrollIntoView` and `SetCaretOffset` as surely as a check box
invites a click. Both halves of that are one change: this crate declares the action, and
`pdf-viewer`'s `App` carries it out as `Command::Scroll` or `Command::Activate`. It was not taken
here because it is a *host* change rather than a reading of a clause, and this round's budget went
to the two entries the clauses support.

**A `/StructParents` array shorter than the page's sequences** stays as ADR 0325 recorded it, and now
has a second reason to stay: it is the case the rejected optimisation above would have lost.

**Whether a stated `/BBox` should win over the shapes that were drawn** is still unmeasured.

## The lesson

**A predicate written for one consumer is not a shared definition, and the way to find out is to
print what it answers.** `select::joins` is right about a highlight and was wrong about a line, and
the difference is invisible in the code, invisible in a unit test with tidy glyph boxes, and obvious
the moment an instrument prints `In`, `terna`, `tiona`, `l `. The count of lines alone would not have
shown it either — 1 004 514 looks like a large document rather than like a defect. What showed it was
printing the longest line, which is now what the instrument prints.
