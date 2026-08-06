# ADR 0212 — A refusal that can become a question, and a bit that means nothing before revision 3

Status: accepted, 2026-08-06 (session 373).

## Context

The project owner stated a policy in the three-hundred-and-fifty-eighth session, and it is in
`CLAUDE.md` under "A document's restrictions are the reader's to set, and they have levels":

> DRM restrictions are low priority and we should always have the possibility to turn them off. We
> should not implement a UI for them right now, but our finishing product will have a few different
> DRM levels: off, on, ask before operations, warn before operation. I tell this, so that, when we
> encounter them for any reason, they are now implemented in a way, which allows such levels later.

Three places in the tree read what a document asserts about its reader, and `doc/todo/38` had them
tabulated:

| clause | what it stopped | where |
|---|---|---|
| §12.8.2.2's `/DocMDP` `/P` 1 | a person typing into a field | `view::permits_form_filling`, at the top of `ViewState::set_field` |
| §12.8.6 / Table 258's `/UR3` | nothing — but a save beyond the grant **withdraws the signature** | `ViewState::save` |
| §7.6.4.2's Table 22 | **nothing** | `Document::permissions` carried the flags and no operation consulted them |

The first was hard-coded to *on*, in the wrong crate, and — the part that decides this ADR — it
refused by **returning zero**. `set_field`'s own doc comment listed what zero meant: the document
has no field of that name, *or* every widget of it is Table 227's `ReadOnly`, *or* §12.8.2.2's
`/DocMDP` forbids the change. Three different statements wearing one number, one of which is a
clause and two of which are not.

That is the whole problem, and it is not a matter of where a boolean lives. Two of the owner's four
levels — *ask before the operation* and *warn before the operation* — need the operation to be
**describable to a person before it happens**. A function that answers `0` has thrown away
everything such a question would be made of: which clause, which level, which operation, which
document. No amount of plumbing a level down to that function fixes it, because by the time the
answer exists the reasons are gone.

## Decision

### The reading and the verdict are separated, and they live in different crates

**`pdf_model::restriction` reads and never decides.** `asserted(&Document, Operation) -> Vec<Restriction>`
answers what the *file* says about one operation, with `Restriction::Certified { level }` naming
§12.8.2.2's Table 257 level and `Restriction::AccessDenied { bit }` naming §7.6.4.2's Table 22
position. Nothing in `pdf-model` refuses anything any more: `ViewState::set_field` applies the value
it is given, and `add_markup` adds the annotation.

**`viewer_core` holds the policy and asks it once per operation.** `Command::Restrict(RestrictionLevel)`
is the one policy value, supplied by the host — §0's rule 2, "the host supplies what the core cannot
know", and how much a person's own program obeys somebody else's file is exactly that. `Viewer::edit`
asks `Viewer::refusal` before resolving the edit, which is **once per `Edit`** rather than once per
widget: §12.7.4.1 makes one field's value shared by all of its widgets, and a question asked per
widget would ask a person the same thing three times about one keystroke.

**The refusal leaves as an event carrying the operation.** `Event::Refused { document, operation,
notes }`, worded by `viewer_core::notes::refusal` in the same voice as everything else this program
says about a document. It is deliberately **not** `Event::Reported`: that event says what the
*document* could not do, and telling a person a file is defective when what happened is that their
own program obeyed it would be a false statement about the file.

### Two levels ship and two do not, and that is the argument rather than an instalment

`doc/todo/38` forbids shipping "a level enum with one caller", on ADR 0178's lesson that a model
entry with no consumer goes stale. The instruction it pulls against is the owner's four levels. The
resolution is that `off` and `on` are answers this crate can give and *ask* and *warn* are **not
answers at all** — they are a question put to a person, which needs a host able to answer it. Two
variants that nothing produced and nothing handled would be two levels silently behaving like a
third, which is `CLAUDE.md`'s "no placeholder implementations".

What makes them cheap to add later is that **nothing in this vocabulary is `#[non_exhaustive]`**.
The day a host can ask, two variants arrive and every consumer fails to compile until it says what
it does about them. That is what §0 keeps these enums exhaustive *for*, and it is the reason the
shape was what this round owed and the levels were not.

The shape is already the one the question needs, which is checkable rather than asserted:
`Event::Refused` carries the operation, so a host receiving it has everything a prompt needs, and
*ask* is that event plus a `Command` carrying the verdict — the shape `Event::PasswordRequired`
already uses, which is why `doc/todo/38` named it.

### Table 22 starts being consulted, for two operations, and the revision decides one of them

**Yes, this is the round.** §7.6.4.1 states the obligation on a reader and it is a `shall`:

> PDF readers shall respect the intent of the document creator by restricting user access to an
> encrypted PDF file according to the permissions contained in the file.

`Document::permissions` has carried the flags since the twenty-second session and nothing consulted
them, which made that `shall` unmet in silence — and made the row *look* fine, because the flags
were read correctly. The reason to pay it now rather than later is that the policy this round builds
is what makes obeying it safe: `CLAUDE.md` says a restriction must always be possible to turn off,
and until this round there was nowhere for that switch to be.

Which flags, for which operations:

- **Filling in a form field**: bit 6, or bit 9 where the revision is 3 or greater.
- **Adding an annotation**: bit 6.
- **Nothing else.** Printing (bits 3 and 12) and assembling (bit 11) name operations this program
  does not have — a capability rather than a permission, and no level would turn one on. Copying
  (bit 5) is *left open* and is the honest gap: what crosses this boundary is a readback, the same
  query answers a drag that merely shows a selection, and Table 22 itself carves the bit for
  assistive technology ("for the limited purpose of providing this content to assistive technology,
  a PDF reader should behave as if this bit was set to 1"), so an operation named `Copy` has to be
  distinguishable from §14.9's tree at the point it is asked — which only a host can do.
- **Saving is not one of them**, and that is a reading rather than an omission: §7.6.4.1's list of
  operations "user access can be controlled" over does not contain it. What a save owes is
  §12.8.2.3's withdrawal, which is below.

**And bit 9 means nothing before revision 3, which this tree would have got wrong.** Table 22 marks
positions 9, 11 and 12 "( Security handlers of revision 3 or greater )", the standard says outright
that "[w]hich bits shall be meaningful … shall depend on the security handler's revision number",
and positions 13 to 32 "must be 1". So in every conforming revision-2 file bit 9 is *set by the
reservation* — including the clause's own example, `/P` −44, which it says "permits printing and
copying but disallows modifying the contents and annotations". A reader consulting bit 9 there would
permit exactly the form filling the example disallows. `Permissions` therefore carries `/R` now, and
`bug900822.pdf` — `/R 2 /P −60` — is the corpus's witness.

### What is not policy, and stays where it is

§12.8.2.3's `/UR3` withdrawal is untouched. A usage rights signature *grants* rather than restricts
— §12.8.6's own note is that "a PDF processor may not permit saving documents by default", and this
one has no feature behind such a gate — so it never appears as a `Restriction`. What it does appear
as is an obligation on a program that **writes**: a save exceeding the grant removes the `/UR3`,
because §12.8.6 makes a usage rights signature the one "referred to from the UR3 entry in the
permissions dictionary" and leaving it would make the file assert something untrue about bytes
nobody signed.

The distinction is in the types rather than only in the prose: `RestrictionLevel` reaches
`Viewer::refusal` and nothing else, and `withdrawn_usage_rights` is reached from `ViewState::save`
with no policy in scope at all. **Turning a restriction off is the reader's; making the file lie is
not**, and `the_reader_can_turn_a_documents_restrictions_off` asserts the second half by reopening
the saved bytes.

## Consequences

**Measured, over the whole corpus, by running `asserted` rather than by reading flags**: 968 of the
974 documents open; **7 assert something against one of the two operations**. Six are encrypted —
`bug1815476.pdf` and `secHandler.pdf` withhold annotating alone, `issue17215.pdf`,
`issue19484_1.pdf` and `issue19484_2.pdf` withhold both, and `bug900822.pdf` withholds both *because
of the revision rule* — and the seventh is `xfa_filled_imm1344e.pdf`, whose certification signature
states `/P` 2 and therefore permits the form to be filled in and forbids a comment on it. 26 carry an
`/Encrypt`, 19 open, and 4 of those open as the owner, whom §7.6.4.1 exempts.

**Nothing a corpus gate draws can move**, and that is worth stating rather than assuming: every one
of those gates interprets a page with an empty `ViewState`, and a restriction is only ever consulted
when an `Edit` arrives.

**`--ignore-restrictions` is the flag, and it is not a user interface.** `viewer-ui` supplies the
policy the way it supplies every other one it has — the sandbox, the backend, the page to open at —
before the document is opened, because a policy applied halfway through is not a policy. The window
prints the reason and the way out. The menu, and the two levels it will offer, are `doc/todo/38`'s.

**One test was rewritten rather than deleted.** `a_certified_document_refuses_the_change_its_author_forbade`
asserted that `set_field` returned 0; it is now
`a_certified_document_states_which_operation_its_author_forbade` and asserts which clause and which
level withhold which operation — including the level-2 case that separates the two, which the old
shape could not express. Both new `viewer-core` tests were checked by disabling their route: the
refusal fails when `refusal` always permits, and the turning-off fails when `Command::Restrict` is
ignored.

**What is left open** is in `doc/todo/38`: the two levels and any interface for them, Table 22's
bit 5 and the copy operation a host would have to name, Annex O's `ef` — whose "[s]ecurity should be
strongly considered when opening an embedded file … a PDF processor may choose to prompt the user or
even prevent opening of the file" is the same four levels arriving from `doc/todo/39` — and whether
the level should ever be per document rather than per reader.
