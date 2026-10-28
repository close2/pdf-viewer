# 2506 — A stimulus activates a hybrid form's calculations in the order of page 379, and they cascade by the names their scripts resolved

Session 1836, a clause round in batch 124. Status: accepted. ADR 2507 holds the limits that end
an activation.

## Context

ADR 2482 built the `FormCalc` interpreter and ran each calculation once, in document order, and
named what it left: "the activation order of page 379 … the cascade by dependency of pages 379–380
with the step budget as its end, the `override` conditions of a calculation, and the result". XFA 3.3
is cited by page and paraphrased (RFC 0011's preamble). `pdf-model` does not depend on
`pdf-formcalc` (ADR 2482 item 5), and the runner a host supplies is the shape ADR 1591 gave
`pdf-script`'s.

## Decision

1. **A `Runner` runs one script; `Resource::activate` decides which and when.** The trait takes the
   script text, the container it belongs to and the steps the activation has left, and returns the
   value, the steps taken and a refusal sentence. `pdf-model` reads no script language. The adapter
   to `pdf-formcalc` is the one the tests and the census already include
   (`crates/pdf-formcalc/tests/common/hosted.rs`), now with `run_counted`, which reports the steps a
   run took.
2. **The order of one stimulus** (page 379: events, then calculations, then validations; page 404
   for scripts the standard leaves unordered):
   - A person's entry: the field's `change` events, then the calculations that depend on the value,
     then the validations. Page 391 says a `change` event is the person's own, so a calculation or
     a merge never fires one.
   - The completion of a merge: every calculation, then the validations, then the `initialize`
     events. **Page 379's general order puts events first; page 407's rule 3 is the rule written for
     this stimulus and puts the `initialize` events last. This ADR follows the specific rule over
     the general one**, so an `initialize` script reads the calculated values. The two pages do not
     contradict each other if rule 3 is read as the exception for the merge; a reader who takes
     them to contradict has the cost that an `initialize` script sees values a calculation has not
     yet written, and that is the departure to revisit.
   - Value calculations before property calculations, and among property calculations the inner
     subform before the outer (page 405, rule 1, items 4 and 5).
   - Scripts of one event run in document order. Pages 404–405 call the order undefined and report
     that Adobe's processor runs simple scripts in document order; that is a choice, not a reading.
   - Only an event of the container itself runs (`ref` absent or `$`, page 1539); an event that
     names another container is not run by this activation.
3. **The cascade follows the names a script resolved, not its text** (pages 379–380). Each
   calculation is run once on a copy of the form to learn what its path expressions returned
   (`Form::resolve` records them); after each real run the set is replaced by what that run
   resolved, so a dependency follows the path the script actually took. A change to a container
   wakes the calculations that named it, and a subform counts as changed when a container inside it
   does (page 379). A calculation never wakes itself: reading one's own value is the circular
   reference page 380 leaves to the form's creator.
4. **The work list is a set taken smallest first, with no recursion.** A calculation runs again
   after a change by being put back into the set. Its key puts value calculations first and then
   property calculations inner before outer, and among value calculations a position in an order in
   which **the calculations a calculation reads come before it** (Kahn's algorithm over the names
   discovery found, taking the template's order among those that are ready and, where only a cycle is
   left, the first of it in the template's order). Ordering by the template alone is correct
   (the cascade re-runs what went stale) but quadratic on a chain whose links come in reverse: each
   link is run once for every link after it, 5 150 runs for a hundred links (a hundred of them
   discovery, 5 050 computing) where the dependency order runs 200 (one of each per link). Both give the same values; only the
   dependency order gives them in time proportional to the form
   (`a_long_chain_in_the_wrong_order_runs_each_link_once_on_its_inputs`).
5. **`override` (page 641–642), absent meaning `error` as the page says.** `disabled` switches the
   calculation off whatever happens; `error` and `ignore` replace a value a person entered in the
   calculated field with the calculated one; `warning` leaves the person's entry standing at the
   entry itself, and a later change to a container the calculation reads runs it again, because the
   person chose to override one value and not to detach the field from its inputs. That last clause is
   a choice: page 642 describes the dialogue (dismiss or override) and a viewer without that dialogue
   has to decide what an override means for the next change. The messages page 642 asks for
   (an error for `error`, a warning with its dismiss-or-override choice for `warning`) are the host's
   to present; the activation says in its report, once per field, that the override applied and that
   the host owes the message, and offers no flag a host could act on, which is owed.
6. **A script assigns text the way a person types it.** A number a script assigns is written as
   decimal text of at most eleven fractional digits without trailing zeros (the precision chapter 25
   displays a result at, page 1062) and is a value like any other typed one: it reaches `/V` and the
   `datasets` packet through ADR 2396's save, and the activation's result is a list of
   `ScriptEdit::Value`.
7. **What this activation does not run is refused by name and reported.** A calculation or event in
   JavaScript, in another language, or in `FormCalc` outside the subset the adapter's interpreter
   knows, is not run and not silently dropped: the report names the field, the activity and why. The
   validations are those of `Resource::before_save`, run on the settled form as a save runs them
   (ADRs 2450, 2474).

## Consequences

- The row for §K.2 stays `partial`: the order and cascade run for FormCalc; owed are JavaScript
  calculations and events (step 5), the events beyond `change` and `initialize`, validations that
  are scripts, and the window's runner (a confined worker host).
- Habit proposed: **a work list ordered by the template is quadratic on a reversed chain; order it
  by dependencies first.**
- A fuzz target, `formcalc_cascade`, holds the activation to its limits, to the drop of edits at a
  limit and to determinism.
