# ADR 0167 — A field has two names, and the answer could only carry one

Status: accepted, 2026-08-03. Session 214. Found by `doc/todo/01-ledger-partial-rows.md`'s third
sweep, run because the round added a verb.

## What the sweep said

§14.9.3's row ended:

> Table 232's `/TU` alternative *field name* is not read: it names a field in a user interface this
> program does not have, which is §12.7.4.1's row rather than this one's.

That is ADR 0122's shape exactly — a row explaining itself by naming something the *program* lacks
rather than something the *standard* leaves open — and it stopped being true in the
hundred-and-thirty-second session, when `pdf-viewer.rs` became a host. (The table number was wrong
too: `/TU` is Table 226's, common to all field dictionaries; Table 232 is the widget's own.)

## What the clause requires

§14.9.3, in the middle of a subclause about alternate descriptions:

> An alternative name may be specified for an interactive form field (see 12.7, "Forms") which, if
> present, shall be used in place of the actual field name when an interactive PDF processor
> identifies the field in a user-interface. This alternative name, if provided, shall be specified
> using the TU entry of the field dictionary.

A `shall`, addressed to a processor that identifies a field in a user interface. Table 226 says the
same thing from the other end — "[a]n alternative field name that shall be used in place of the
actual field name wherever the field shall be identified in the user interface (such as in error or
status messages referring to the field)" — and adds the accessibility motivation §14.9.3 is filed
under.

## What actually blocked it, which was not the window

`Query::FieldAt` answered `Answer::Field(String)`: §12.7.4.2's fully qualified name, documented as
"[w]hat a host needs before it can send `Edit::SetField`". One string, and the two jobs it is
wanted for are not the same job.

- **Identity.** `ViewState::set_field` addresses a field by its qualified name, because §12.7.4.1
  makes the value the *field's* and §12.7.4.2's name is what says which field. §12.7.6.2's export
  wants the same string. A `/TU` here would address nothing.
- **A label.** §14.9.3's sentence is about what a person is shown.

So whichever meaning the one string took, the other was lost at the caller, and a host had nothing
to obey the clause *with*. The capability that was missing was the answer's shape rather than the
window — which is why the row survived the arrival of the window that was supposed to expire it.

`Answer::Field` now carries `pdf_model::view::FieldName`: `qualified`, `alternative`, and
`shown()`, which is the clause's own choice between them. Handing both over rather than choosing
here is deliberate — this crate does not know whether its caller is addressing the field or naming
it to a person, and a type that guessed would be wrong half the time in silence.

## Where `/TU` is read from

**Table 226 does not mark it inheritable**, and it marks `/FT`, `/Ff`, `/V` and `/DV` so — which
decides the lookup. `/TU` belongs to the *terminal field* and to no ancestor of it: the widget's
own dictionary where §12.5.6.19's merge applies, and its `/Parent` where the widget is a kid with
no `/T` of its own. `alternative_name` climbs `/Parent` to the first dictionary stating a `/T`,
bounded by `MAX_FIELD_DEPTH` and guarded against a cycle, which is the same distinction
`widgets_by_field_name`'s walk makes coming down the other way.

## Evidence

Eighteen of the 974 corpus documents contain the bytes `/TU`, which is a lower bound because an
object stream hides them. `issue17492.pdf`'s first widget is the merged form: `/T (firstName)` with
a `/TU` in UTF-16BE, so the test also takes §7.9.2.2's other encoding through the same path.
`viewer-core/tests/headless.rs::a_field_states_the_name_a_user_interface_is_to_show` asks for the
field at the middle of the widget's stated rectangle — from the document, not from the code under
test — and checks that the identity is still `firstName` while the shown name is `First name`.

`form_two_pages.pdf` states no `/TU`, and the existing test now asserts that too: the clause's "if
present" case is where a stand-in would be invented, and this one falls back to the field's own
name rather than to anything.

## What is still owed

Nothing displays a field's name yet — `viewer-ui` fills in no forms, and `tests/headless.rs` is the
only consumer of `Query::FieldAt`. So this closes the clause's reachability rather than a visible
defect, which is worth saying plainly: the finding is that the *interface* made a `shall`
impossible to obey, and that is fixed. When a host does show a field, `shown()` is what it calls.
