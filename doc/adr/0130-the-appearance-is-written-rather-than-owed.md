# ADR 0130 — The appearance is written, rather than owed to the next reader

*Session 145. Closes the second of ADR 0121's two costs.*

## Context

ADR 0121 wrote down two costs of §7.5.6's incremental update. ADR 0129 closed the first. This is
the second, and it was stated like this:

> A widget's stored appearance is not regenerated — Table 224's `/NeedAppearances` is set
> instead, so a reader that ignores the flag shows the old value.

The argument for the flag was not lazy, and it is worth restating because this decision is partly
against it: regenerating means writing content streams into somebody else's file, and what would
be written is this program's own reading of a clause. Table 224 exists precisely so a processor
that would rather not can say so.

## What changed the answer

**This program already constructs the stream.** `appearance::regenerate` splices §12.7.4.3's
`/Tx` marked-content region into the stream the file carries, and `appearance::construct` builds
one from Table 192's characteristics where there is none. Both run on every render of an edited
field. So writing the appearance is not new code deciding what a field looks like — it is the
*same bytes* the viewer just drew, put where the next reader will find them.

That reframes the cost. The risk of writing a content stream into someone's file is that it
disagrees with what the file says; here it cannot, because the alternative to writing it is
drawing it, and this program does that either way. What the flag buys, meanwhile, is a promise
that every reader honours it — and §12.7.2 says what setting it *admits*:

> If such an object defines an appearance stream, the appearance shall be consistent with the
> object's current value as a field

A file where every changed widget carries a correct stream is a file where that obligation is
kept. Setting the flag anyway would be saying otherwise.

## Decision

`ViewState::save` writes the appearance stream for every widget whose value it writes, and sets
`/NeedAppearances` **only** for the widgets it could not.

Three things this needed, and each is a rule rather than a detail.

**One: §7.5.6's update may add objects, not only replace them.** The clause's own list is
"objects that have been changed, replaced, or deleted", and a widget with no `/AP` needs an object
*added*, because §7.3.8.1 makes every stream indirect and there is nowhere else to put one.
`Update::beside` picks the first free number from **both** the trailer's `/Size` and the highest
number the cross-reference table actually holds, taking the larger. §7.5.5 says `/Size` shall "be
1 greater than the highest object number defined in the PDF file"; **68 of the corpus's 974 documents write
at least one entry beyond their own `/Size`**, a fact this tree already records in the §7.5.5
ledger row for a different reason. Trusting `/Size` alone would put a new object on top of an
existing one, silently.

**Two: the stream that was there keeps its dictionary and its object number.** `/Matrix` and
anything else its producer stated survive; only the bytes and what describes the bytes —
`/Length`, `/Resources`, and the `/Filter` that no longer describes anything — are replaced.
Replacing rather than adding also means the update does not orphan the old stream.

**Three: only the field types §12.7.4.3 covers.** `appearance::regenerates` already decides which
those are for the drawing path, and `for_saving` asks it rather than restating the reasoning. Its
`ForSaving::Selected` arm is a check box or radio button, whose states §12.7.5.2.3 makes
"defined by an appearance stream in the appearance dictionary of the field's widget annotation"
and whose *value selects among them*. A writer that regenerated those would replace the
subdictionary of states with one stream and lose the off state — a file that renders correctly
everywhere until somebody clicks the box.

## What is checked, and one thing the first check missed

`crates/pdf-model/tests/saving.rs`, three tests, each on a corpus document:

- a text widget with no `/AP` at all (`160F-2019.pdf`, object 417) is given one: a form XObject
  with §8.10.2's entries, `/Tx BMC` inside it, and the typed value;
- a check box whose `/MK` states a background and a border (`bug1675139.pdf`, object 50) keeps its
  state subdictionary when it is the field *being changed* — the fixture had to be one that draws
  something, because a check box stating no background and no border produces no stream either
  way and would have passed for the wrong reason;
- `form_two_pages.pdf` saved does not set `/NeedAppearances`.

**The check box's assertion was wrong the first time and the guard's removal is what showed it.**
It read `normal.as_dict().is_some()`, and `Object::as_dict` answers for a *stream* as well as a
dictionary — so a `/N` replaced by a stream passed it. With the guard removed the states really
were being destroyed and the test said nothing. It now matches on `Object::Dictionary` and fails
as it should. That is this tree's own habit — a test asserted through the accessor that normalises
the thing being tested is not a test — met one level down, in a library accessor rather than in
one of ours.

`viewer-core`'s end-to-end save test gained the assertion that makes it discriminate too: with
`/NeedAppearances` unset, a second viewer reading "Ada Lovelace" back can only be reading the
written stream, where before it was honouring the flag this program had set.

Outside the tree: `pdftotext` and `mutool` both extract the typed value from the saved
`form_two_pages.pdf`, and `mutool show` of the widget's stream shows the splice doing exactly what
§12.7.4.3 describes — the producer's border artwork above, `/DR`'s three fonts merged into
`/Resources`, and the new `/Tx BMC` region appended because the original stream had none.

## The cost that is left

A widget this program can lay out only part of — a `/DA` naming a font `/DR` does not define, a
composite font it cannot address by character — gets its stream written *and* leaves
`/NeedAppearances` set. Both statements are true and neither is complete on its own: here is what
could be laid out, and it is not all of it.
