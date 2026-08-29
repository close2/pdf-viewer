# 0753 — The field list a reference could not open, and a tie-break that heads into an exclusion

Status: accepted.
Context: the errata selection rule's eighteenth use — the ninth consecutive use whose base count
reproduces the previous use's closing arithmetic, and the second on which all three rankings are
flat.

**Sibling rounds hold the numbers above this one.** `main` carries through 0752 at this round's
base, so 0753 is the tip and was taken on that reservation.

## The rule, and what a third flat use could tell

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step, ADR
0691's writing rule, ADR 0712's placement rule, ADR 0732's family guard and ADR 0743's fifth step.
ADR 0749 concluded from two flat uses that "the rankings are a filter and the tie-break is the
selector" and declined to add a step 6, on the argument that a third *cardinal* would answer a
question the rule has not been failing at. That argument stands and is not reopened here.

**What a third flat use adds is not another ranking but a measurement of where the tie-break
points.** Both row rankings top out at two annotations — six rows over the live rows, 28 over every
row with 17 more at one — and the issue ranking tops out at two with 31 tied there and 23 at one.
The field is the seventeenth use's less its two verdicts. So the tie-break chose again, and this
time the population it chose from is small enough to characterise:

- **Of the 54 unread issues, seven land only on `out-of-scope` rows, and every one of the seven is
  clause 13's.**
- **Three of those seven are the whole of the population's requirement-level substitutions** — a
  `should` for a `shall` (§13.2.4.2), a `should` for a qualified `shall` (§13.2.6.1), an `is` for a
  *may be* (§13.7.2.3.5). The one remaining modal edit anywhere in the field is §7.5.7's `might`
  for `may`, which is ISO house style over a statement of possibility rather than a level.

ADR 0653's tie-break reads *a cell ahead of a word in prose, and among cells the requirement level
first*. So its first preference and `CLAUDE.md`'s largest exclusion now select the same ground, and
the sixteenth use's head being inside that exclusion was not the accident ADR 0746 recorded it as.

## The amendment: step 6, and why it is not a retirement

`doc/todo/01`'s recipe gains a sixth step: **count the issues whose every landing is on a row the
closed exclusion list covers apart, and rank the rest.** It is step 3's family guard applied one
population over — that guard counts an informative annex's annotations separately because the
ledger has no rows there and they would otherwise manufacture a head — and the argument is the same
shape:

- **An erratum inside a closed exclusion can produce nothing but a confirmation.** `CLAUDE.md` says
  an exclusion is revisited "by argument, never by attrition", and an erratum is evidence about the
  standard (ADR 0601) rather than an argument about this project's scope. A `shall` arriving inside
  clause 13 does not make clause 13 a rendering question.
- **They are not dropped.** The column is a column, not a filter: reading one to a verdict is the
  only way it leaves the population, and it costs minutes. This round disposed of two of the seven.
- **What it buys is that the walk starts where it can end.** The rule's own record now shows the
  same shape three uses running: the head confirms or cites, and the payment comes from walking
  downward. Step 6 does not change what pays; it stops the walk beginning in ground where nothing
  can.

**The retirement condition stated by the eight-hundred-and-eleventh is not met** — "the day an issue
read whole stops changing anything". This use's reading changed a line of behaviour, a wrong comment
beside it, a ledger row and two documents. The honest summary of the instrument is narrower and
sharper than "retire it": *the counts have stopped ranking, the tie-break selects, and the tie-break
needed a guard that the counts never did.*

## The payment: Table 241's reference form opened only a leaf

§12.7.6.3's Table 241 gives `/Fields` two spellings — "either an indirect reference to a field
dictionary or (PDF 1.3) a text string representing the fully qualified name of a field" — and Table
242 makes either of them reach further than the field named:

> All descendants of the specified fields in the field hierarchy are reset as well.

`ViewState::reset_form` implemented the **name** form as a prefix test over §12.7.4.2's qualified
names, which is right. The **reference** form was `vec![*id]`: the referenced object identity, used
as a widget identity, under a comment arguing that "what identifies a field here is its object
identity, as it is for §12.6.4.11's annotations."

**The analogy is where it went wrong.** Table 214's `/T` names an *annotation*, so a hide action's
reference is the thing to be hidden. Table 241 names a *field*, and §12.7.4.1 merges a field
dictionary with a widget annotation only where the field has exactly one widget. So:

- a reference to a **non-terminal** field reset nothing at all, and none of its descendants;
- a reference to a **terminal** field whose widgets are separate `/Kids` entries reset nothing
  either, because the field dictionary is not one of them.

Both are documents ISO 32000-2 describes, and this reader answered both by drawing the value the
action said to reset.

`view::widgets_under` walks the referenced field's subtree to its leaves, under the same
`MAX_FIELD_DEPTH` bound and the same `seen` guard `walk` carries — `/Kids` is the document's to
write and §12.7.4.1 states no acyclicity rule. A leaf is kept whether or not §12.7.4.2 gives it a
name: the caller wants the annotations to reset rather than the fields to list, and a `/Kids` entry
with no `/T` of its own is exactly the widget of the field that was named. The name form is
untouched, and `widgets_by_field_name` — which the hide action, the form panel and the censuses
all read — is untouched with it.

### What Issue #174 contributes, which is the reason this was read at all

The published Table 242 puts the descendant parenthesis on the **clear** branch alone; its *set*
branch says only "all fields … shall be reset except those listed in the Fields array". Issue #174
appends the mirror sentence — *(All descendants of the specified fields in the field hierarchy are
also exempt from being reset.)* — so both branches are one subtree question. The set built here is
shared by the two branches, so the behaviour was already symmetric; what the erratum supplies is the
warrant. Under the published text a reader would have had to argue the asymmetry was an oversight.

### Issue #683 beside it, which vindicates rather than moves

The same page carries a strikeout over `; inheritable` in Table 241's `/Flags` cell, leaving
*(Optional)*. `action::reset_form` reads `/Flags` off the action dictionary and follows no chain,
which the published cell called wrong — and there was never a chain: inheritance is §7.7.3.4's page
tree and §12.7.4.1's field tree, and an action dictionary is in neither. The live comment is
corrected to say so; the analogy it drew to §12.6.4.11 is what carried the defect above, so the two
corrections are one sentence apart on purpose.

## Counted before believed, and the corpus is silent

`crates/pdf-model/examples/reset_form_census` is new and reproduces §12.7.6.3's row's own figure —
three reset-form actions in two documents — while adding the question that matters: **every
`/Fields` element in the corpus is a fully qualified name, and not one is an indirect reference.**
So no gate on this disk could have seen the defect and none can see the fix, which is trap 8 exactly
and the answer §12.6.4.2's `/SD` row already gives: a fixture that differs from its neighbour in the
one entry.

## Calibration

Trap 13, above the commit that makes the change, in two directions, each failing a different
assertion of `a_reset_form_action_named_by_reference_reaches_the_fields_descendants`:

| planted | the test says |
|---|---|
| `ResetTarget::Field(id) => vec![*id]`, the defect as found | `left: (false, false, false)` against `right: (true, true, false)` — "a reference to a non-terminal field resets the widgets under it" |
| `descend` pushing every node rather than only a leaf | "the non-terminal field is not itself a widget" — the field dictionary reaches `is_reset`, which no page ever asks about |

## The blindnesses were two lists under one word

`doc/errata-read.md` is where the blindnesses live, and read in order it numbers them *first*,
*second*, *third*, *third*, **eighth**, *fifth*, *sixth*. That is not a list that lost count. There
are two of them, and no document said so:

- **the instruments' list** — what `spec-errata check` and `emit` cannot see. Six: an addition over
  unquoted text, a strike under the four-word floor, a spelling `doc/md/` writes differently, a pure
  insertion into a sentence quoted correctly, an erratum whose substance is an attached file (ADR
  0736), and a table's number rather than a clause's (ADRs 0746, 0750), the last closed by
  `spec-errata renumbered`;
- **the rule's own list**, wider, counting erratum-shaped blindnesses beside the instruments' and
  reaching eight by the seventh use, whose eighth is step 2's grep being unable to tell a *use* of an
  issue number from a *mention* of one (ADR 0691).

The cross-references already run between them: ADR 0749 cites "ADR 0691's fourth blindness", and
ADR 0691 is where the *eighth* was recorded. **Nothing is renumbered**,
here or in any ADR (ADR 0232 §2) — what is added is the sentence saying they are two, and a rule for
what comes next: a new blindness is numbered on the **instruments'** list, because that is the one a
command can be written against; a blindness that is the rule's rather than an instrument's is
described and not numbered.

This is the decay `doc/errata-read.md`'s own *Owed* list took two rounds before this one, for the
identical reason: **a claim about the instruments has nothing sweeping it**, and the file that holds
it is not in any sweep's population.

## Consequences

- `ViewState::reset_form` performs Table 242's descendant rule for both of Table 241's spellings.
  `view::widgets_under` is the walk; `widgets_by_field_name` and its six callers are unchanged.
- §12.7.6.3's ledger row records the defect, the fix, both errata and the census.
- `crates/pdf-model/examples/reset_form_census` exists.
- `doc/todo/01`'s recipe gains step 6; `doc/errata-read.md` carries the four annotations, the
  exclusion measurement and the blindness index.
- Two clause-13 errata have verdicts and are out of the population; five remain in it, priced.
