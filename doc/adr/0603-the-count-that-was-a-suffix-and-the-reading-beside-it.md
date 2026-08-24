# ADR 0603 — The count that was a suffix, and the reading that has to sit beside it

Status: accepted, 2026-08-25. Session 721, the eighth round on the project owner's *"we should
start investing time into the UI (and its API for the native versions)"*. ADR 0509's ordering was
spent in the seven-hundred-and-seventeenth, and `doc/todo/30` named two items in its place; this
round takes the second — *sort `tools/state.sh windows`' unreached list into debts and non-debts,
and record the reading beside the count*.

It could not be sorted as it stood. Two of the entries were artefacts of how the count is taken and
two genuine gaps were hidden by the same two flaws, so this ADR is in two parts: what the
instrument was measuring, and the reading it can now carry. ADR 0604 is the debt the reading found.

No message was added and no variant changed shape — the thirteenth consecutive round since the
six-hundred-and-seventh in which that has been true. `viewer-core` was not touched.

## 1. Why an uninterpreted count is worse than no count

ADR 0577 built `tools/state.sh windows` because ADR 0564 §7 had written down, in prose, that
§12.3.5's collection and §12.5.6.14's popups were still `viewer-ui`'s alone and that *nothing
counted it*. It came with a note saying, correctly, that a zero there is not automatically a
debt — `Query::Dirty` is reached by no window because all three learn about an edit from
`Event::Dirty`.

That note is where this round starts, because the two rounds after it did with the number exactly
what an uninterpreted number invites. Both wrote *"eleven queries each native host does not ask"*
and neither said which of the eleven a reader would notice. A figure that cannot be acted on is
read as a figure that has been dealt with: the parity claim "all three hosts stay level" now had an
instrument pointing at it, and the instrument's output was a list nobody could rank.

**So the reading is the deliverable, and it goes in the script rather than in a document.**
`CLAUDE.md`'s rule is that a fact a command can print is not written down and the command that
prints it is; a *reading* is the other kind of thing — no command can decide whether a `GtkEntry`
owning its own caret is a gap or a delegation — so it is written down, and it is written down where
the number is, because that is the only place it will be read at the same moment.

## 2. The instrument was wrong twice, and both are trap 11

### `PathCommand::Close` is not `Command::Close`

`names_in_code` matched `grep -oE "Command::[A-Za-z]+"`, with no word boundary. `pdf_render`'s
display list has a `PathCommand` enumeration whose `Close` closes a *path*, and `viewer-ui` writes
one on every rounded rectangle of its own chrome — so the question *"does this window ever close a
document?"* was answered by a piece of chrome geometry, in a file about drawing panels.

The general rule is worth more than the instance and it is trap 11's own: **a grep for an
enumeration's variant is a claim about a path through the source, and a suffix is not one.** `\b`
is the whole fix.

### A trace formatter's match arm is a name printed, not a question asked

`section_hosts` asks `viewer-ffi` alone, and its comment says why in so many words:

> **Only `viewer-ffi` is asked, deliberately.** Every `Command::` and `Query::` in that crate is a
> call: it has no trace module and no wire protocol, so naming a variant there means offering it.
> `viewer-ui` names all of them in `trace.rs` and `viewer-confined` in its protocol, so the same
> grep over those two would answer 100% and mean nothing — trap 11's shape, a count whose condition
> is not the question.

`section_windows` was then written sixty lines below it, over a population containing `viewer-ui`.
It excluded `viewer-confined` and named that same comment as its reason for doing so, and did not
exclude the trace formatter the comment's other clause is about. **The condition was documented and
not applied, in one file, by one round.** That is a sharper failure than the original trap 11
instance and it belongs beside it: a rule written down in the paragraph above the code is not a rule
the code has.

`viewer-ui`'s `trace.rs` matches `viewer_core::Command` exhaustively in order to print a command's
name. The host sends twenty-two of the twenty-five.

### What the two of them cost

`Command: viewer-ui reaches 25 of 25`, and under it `every Command reaches at least one window`.
Both false. With the word boundary and the trace formatter out:

- `viewer-ui` reaches **22** of 25 — `Close` and `Focus` were the suffix and the formatter,
  `Delegate` the formatter alone;
- and **no window reaches `Command::Close` or `Command::Focus`**, which is a line the section is
  built to print and had never printed, because the two things it was counting were a path close and
  a `format!`.

## 3. The reading

Fifteen `Kind::Variant` rows, each `debt` or `not a debt` with the reason attached, printed under
the counts by `say_the_reading`. The full text is in the script; what belongs here is the shape it
falls into, because that is the finding rather than the list.

**The non-debts are three kinds and not one**, and ADR 0577's note named only the first two:

1. **Learned another way.** `Query::Dirty`: every window takes it from `Event::Dirty`.
2. **The tier.** `Query::Frame` for `viewer-ui`, which draws its own pixels and hands the viewer
   none, so the answer would be `Answer::None`.
3. **A delegation** — and it is four of the eleven rather than a vague "most". `Query::Caret`,
   `Query::Offset`, `Query::FieldSelection` and `Query::FieldAt` are all what a host asks when it
   draws §12.7's field itself; a host that placed a real `GtkEntry` over the widget has the
   toolkit's caret, the toolkit's hit test and the toolkit's in-field selection, and asking would be
   a second answer to a question already answered.
4. And one this round adds: **a message whose precondition no window meets.** `Command::Close` and
   `Command::Focus` address a *second open document*, and every window in this tree opens one file
   from its command line and lives as long as it. They are `Query::Collection`'s companions — a host
   presenting §12.3.5's collection would hold two `DocumentId`s — which is why the reading names
   them together rather than shrugging at a zero.

**The debts are five, and they rank**, which is what the exercise was for:

| | what a reader loses | clause |
|---|---|---|
| `AccessibilityTree` + `Readback` | a screen reader on either native host is handed a picture | §14.7, §9.10.2 — `doc/todo/31`'s |
| `Popups` | a comment on the page is invisible in two windows of three | §12.5.6.14, Table 186's `/Open` |
| `Collection` | a portable collection is not presented | §12.3.5 — a `shall` addressed to a viewer |
| `LinkAt` | a link cannot be seen to be one before it is clicked | none — a convention one host has |
| `FreeTextAt` | authoring a free-text annotation | §12.5.6.6, and **already refused by name** |

The last row is why the verdict column says more than *debt* or *not a debt*: a capability two hosts
decline **out loud** (ADR 0526) is a different thing from one nobody has noticed, and a table that
called both "a debt" would have lost the distinction this project spends most of its care on.

## 4. What keeps the reading from becoming the thing it replaced

A table of reasons in a script is a document with better placement, and documents decay. So the
section checks it in **both** directions and says so in the output:

- a variant no window reaches with **no row** prints `UNREAD — this round owes a reading`;
- a row for a variant every window now reaches prints `SPENT — this reason has outlived it`.

Both were run against injected defects before being believed (trap 13): a row renamed produced
`UNREAD` for the variant and `SPENT` for the renamed row in the same run.

The second direction is the one that matters here, and this round is its first customer:
`Command::Restrict` was on the native hosts' missing lists when the round began and is not on them
now, so its reason had to be deleted rather than left to describe a debt somebody closed. That is
the ledger's own failure mode — a row describing what the code *should* do — with an instrument
against it.

## 5. What this ADR deliberately does not do

It does not rank the five debts against each other beyond the table above, and it takes none of the
four it did not close. ADR 0509's criterion still orders them and `doc/todo/30` carries them; the
round that takes one is the round with the compiler and the screenshot.

It also does not extend ADR 0509's numbered list, which the seven-hundred-and-seventeenth session
spent. The criterion outlives the list — what a reader can do and cannot do here, then what costs no
new message, then what makes the level-hosts decision checkable — and what the list was for was
saving three rounds a survey each. This section is that survey, in a form that reruns itself.
