# ADR 0087 — The one form action that is a picture

Status: accepted, 2026-08-01.

## Context

`CLAUDE.md` puts field *appearance* in scope and field *behaviour* out of it, and §12.7.6's three
form actions look like behaviour in its purest form. Two of them are: a submission is a network
request a document initiated, an import wants a filesystem.

The third is not. §12.7.6.3:

> an interactive PDF processor shall reset selected interactive form fields to their default
> values; that is, it shall set the value of the V entry in the field dictionary to that of the
> DV entry … If no default value is defined for a field, its V entry shall be removed.

A field's appearance is laid out from its value (§12.7.4.3, ADR 0032). So a reset is a change of
*which entry the value comes from*, and everything under it has existed since the twenty-third
session.

## Decision

**Reset is performed.** `ViewState` gains a set of widgets a reset has touched — beside
§12.6.4.11's hidden annotations, and for the same reason: nothing is written to the file.
`Field::read` then walks the same `/Parent` chain reading `/DV` instead of `/V`, so a field with
no default anywhere ends with **no value at all**, which is what "its V entry shall be removed"
means for a program that does not write to documents.

A check box answers from its `/DV` rather than its `/AS`, which is the one place the reset has to
override a rule this tree already implements: §12.7.5.2.3 makes `/AS` win over `/V`, and after a
reset the `/AS` in the file describes precisely the state the action replaced.

`/Fields` is read in both spellings and all three shapes Table 241 and Table 242 give it: absent
means every field in the form; present with the flag clear means those fields and "[a]ll
descendants of the specified fields in the field hierarchy", which §12.7.4.2's naming turns into a
prefix test; present with the flag set means everything except them.

## Nine rows that said `silent` while the code was talking

`silent` means "not implemented, and nothing says so". Nine rows carried it while `action.rs` had
been naming its refusals since the sixty-second session — `GoToR`, `GoToE`, `GoToDp`, `Launch`,
`Sound`, `Movie`, `Trans`, `SubmitForm`, `ImportData`, each with its own sentence.

They were not *quite* `reported`, and the missing half was in `viewer-ui`: `perform_all` returned
the refusals and the viewer dropped them, so a click that declined to do something looked exactly
like a click on nothing. Trap 5's rule — unsupported input stays loud — applies to a *click* as
much as to a content stream, and now the viewer prints what it declined and why.

That is the sixth understated-row finding of this run, and the first where the fix was three lines
of user interface rather than a re-reading of a clause.

## Consequences

- `silent` falls 62 → **49**, and `reported` rises 27 → 36. Thirteen rows move, of which one is a
  new feature and nine are the code being described accurately at last; §12.7.9 closes because
  §14.8.5.6 closed it last session, and §12.7.6 and §12.7.6.1 become `partial`.
- §12.6.4's performed actions are now seven, and this is the first one whose effect is *ink on
  the page* rather than which page is shown.
- No gate moves. The corpus's 3 reset actions sit behind clicks, and no gate clicks.
- What clause 12 still owes on forms is the other direction: a person **typing** a value.
  §12.7.6.3 proves the machinery underneath works — a value that changes redraws correctly — so
  what is missing is an editor, not a layout.
