# 1173 — What the output is for is an input a host supplies, and §8.11.4.4's adjustment sets

Session 1168. Status: **accepted**. Revisits [ADR 1106](1106-who-is-reading-is-an-input-and-a-zoom-is-a-factor.md)
§5 and [ADR 0375](0375-the-quotation-mark-the-conversion-changed-and-the-one-no-sweep-could-see.md).

Two clauses ask this processor what the output being produced is *for*, and both answers were
declined on the same unlisted ground — that this program does not print, or that this device is a
screen. §8.11.4.4's `Print` and `Export` usage events were "operations this program does not
perform" (ADR 1106 §5, from ADR 0044); §8.9.5.4's step c) "addresses printing and this device is
a screen, which is why `/DefaultForPrinting` is still read by nothing" (ADR 0375). Neither ground
is on `CLAUDE.md` principle 5's closed exclusion list, which is the definition of an exclusion by
attrition — and the same file forbids one: "[r]evisit an exclusion by argument, never by
attrition."

This ADR makes the answer an input, and then finds that building the input exposed a defect in
the adjustment the input feeds.

## 1. One question, asked once, on ADR 1106's channel

`pdf_model::optional_content::Purpose` is `View`, `Print` or `Export`, and
`ViewState::set_purpose` is the only channel into interpretation — beside `set_magnification`,
`set_audience` and `set_widget_appearances`, under `viewer_core`'s rule 1. `Purpose::View` is the
default, so every existing caller draws the page it drew before, by construction.

It is one input and not two, because the two clauses ask one question. Table 101's `/Event`
"[s]hall be one of View, Print , or Export"; §8.9.5.4 step c) opens "Otherwise if the PDF is
being printed". A renderer that decided either for itself would be answering a question nobody
asked it, which is `CLAUDE.md` principle 3's rule and ADR 1039's refusal repeated.

**Table 101's `/Event` and its `/Category` stay different types**, because §8.11.4.5 NOTE 3 says
they are different things: "[a]lthough the event types Print and Export have identically named
counterparts that are usage categories, the corresponding usage application dictionaries are
permitted to specify that other categories can be applied." So a `Print` event may name `/Zoom`,
and `Purpose` selects the dictionaries rather than the categories.

## 2. The duration is a structure, not a discipline

§8.11.4.5 gives each of the two events a duration: "[t]hese changes shall persist only for the
duration of the print operation; then all groups shall revert to their prior states", and the
same sentence for the export. A reversion a caller performs is a reversion a caller can forget,
and then the clause is met by each caller's diligence rather than by the code — ADR 1106 §3's
argument for putting the reapplication inside `set_magnification`, applied again.

So the event's result is an **overlay**: `OptionalContent::under_event` holds the states as the
event leaves them, `state_of` reads it in preference to `states`, and stating `Purpose::View`
again deletes it. The prior states were never written over, so nothing can drift, and a document
that states no `/AS` pays one empty-vector test.

## 3. The adjustment sets, and a category that read nothing recommends nothing

Building the `Print` event found a defect in the `View` one. §8.11.4.4 states the per-group rule
as an assignment:

> For each of the groups in OCGs , the entries in its usage dictionary … specified by Category
> shall be examined to yield a recommended state for the group. If all the entries yield a
> recommended state of ON , the group's state shall be set to ON ; otherwise, its state shall be
> set to OFF .

`apply_view` implemented it as an AND with the state the group already had, which meant a
usage application dictionary could turn a group off and never on. **The clause's own example is
what settles it, and it settles it on the `Print` event.** The example states `/BaseState /OFF`
with `/ON [1 0 R]`, so objects 2, 3 and 4 begin off; it states
`<</Event /Print /Category [/Print] /OCGs [4 0 R]>>` and gives object 4 a `/Print` usage entry of
`/PrintState /ON`; and it then says outright: "[w]hen printing or exporting, object 4 receives an
ON recommendation." Under an AND with the initial state that recommendation could not reach the
page, because object 4's initial state is OFF. The same paragraph says of the `View` dictionary
that it "specifies that all optional content groups have their states managed based on zoom level
when viewing" — which a rule that could only ever turn a group off would not do for objects 2 and
3 either. The only AND the clause states is across categories and across dictionaries: "its state
shall be ON only if all categories in all the usage application dictionaries it appears in have a
state of ON".

**The assignment is only safe beside a second correction, and the two are one decision.** A
category whose Table 100 entry the group does not state has to recommend *nothing* rather than
ON, which §8.11.4.4's own `Zoom` example writes outright — "Object 4 has none; therefore, it is
not affected by zoom level changes". Under an AND the difference is invisible, because ON and
nothing are the same no-op; under an assignment it is the difference between leaving a layer
alone and switching it on. So `Usage`'s `view` and `export` became `Option<bool>` beside `print`,
which the clause had always stated separately ("[i]f PrintState is not present, the state of the
optional content group shall be left unchanged"), and `Zoom`, `User` and an unrecognised
`/Category` name all yield `Recommendation::Unchanged`.

`apply_event` is the one function, and the events differ in two arguments the clause gives them:

- **the base**, which is §8.11.4.5 b)'s initial state for `View` — so the answer is a function of
  the factors rather than of the order the zoom steps arrived in — and the states as they stand
  for the other two, which are "applied over the current states of optional content groups";
- **what is pinned**, which is the manually changed groups for `View` and nothing for the other
  two, because the sentence names the event it pins against: manual changes "shall not be
  readjusted based on usage application dictionaries with event type **View** as long as the
  document is open". A person who switched a layer on to look at it has not said what should
  happen to it on paper.

## 4. §8.9.5.4 step c), on the same input

> Otherwise if the PDF is being printed and any of the Alternates entries has DefaultForPrinting
> set to true, then that alternate image shall be printed.

`Interpreter::default_for_printing` is it, and two things the amended step does **not** say are
what its predecessor did — so the function must not either. It does not send the selected
alternate back through its own `/OC` (Errata Collection 3 strikes that sentence out whole), and
it states no fallback of its own, because step e) carries the fallback for c) and d) together.
Table 89 is what makes "any of the Alternates entries" identify one: "[a]t most one alternate for
a given base image shall be so designated", so the first is the one, and a document designating
two has contradicted its own requirement and gets the first deterministically — a documented
choice, since the clause states none.

c) opens at "Otherwise", so it belongs to a base image stating no `/OC`: steps a) and b) dispose
of every base image that states one, and printing does not reopen the `/Alternates` that a)
closed. `Purpose::Export` fails c)'s condition and falls to d), which is the clause's own
arrangement — an export is not a printing.

## 5. What moves, measured

`examples/oc_usage_census` over the 963 pdf.js corpus documents that open: six state an `/AS`,
naming `View` six times, `Print` six and `Export` five. Reading their `/OCProperties` directly,
**`bug1650302_reduced.pdf` is the only one where the events disagree** — a `view` layer its
`Print` dictionary turns off and a `print` layer it turns on — and its page content states no
`/OC` at all, so not one pixel of it depends on either. The other five recommend ON for a group
the configuration already has on, or name groups that state no `/Usage` dictionary and are
therefore left alone by section 3's second correction. **So no corpus page moves, on a screen or
in an export**, and the `Print` event has no corpus caller at all yet (section 6).

No corpus document carries `/Alternates`, which both ADR 0092 and ADR 0375 measured, so §8.9.5.4
step c) rests on the clause and on three fixtures.

## 6. What this does not decide, and what remains

**The `Print` event has no caller, and that is a caller rather than a capability.** RFC 0004
designs a print path and none is built; `viewer_host::restriction` says so in its own words
("[p]rinting and assembling are `pdf-transform`'s verbs and no window has one"). What that RFC
needs from this crate now exists: its section 4's "print intent" is `Purpose::Print`, passed on
the render request, and both of the clauses its bullet list names for optional content and
alternate images are carried out the moment it passes one. Nothing else is owed here.

The `Export` event has one, and ADR 1174 is which operations are exports and why.
