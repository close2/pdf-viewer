# 1175 — The actions a target forbids, and the actions that follow them

Status: accepted and **built**. Session 1169.
Context: `crates/pdf-transform/src/archive/actions.rs` (new),
`crates/pdf-transform/src/archive/{decision,prepare,rewrite,report,mod}.rs`,
`crates/pdf-archive/src/table/interaction.rs`, `crates/pdf-transform/tests/archive.rs`.
Answers: `doc/todo/66`'s Actions family and `doc/pdf-a-mitigations.md` section 8, which this moves
from *catalogued* to *built*; the `forms/no-action-on-widget-or-field` row of section 7 goes with
them, because one routine answers all nine.
Builds: `doc/adr/0947` (the three stages and the rule that nothing changes which no failed
requirement asked to change), `doc/questions/A54`–`A55` (the configuration is where an answer comes
from), `doc/adr/1120` (mark provenance is the fence's test).
Clauses: ISO 32000-2 §7.3.7, §7.3.10, §7.7.2 (Table 29), §7.7.4 (Table 32), §12.3.3, §12.5.2
(Table 166), §12.6.2 (Table 196, NOTE 1), §12.6.3 (Tables 197–200), §12.6.4.17, §12.7.4.1;
ISO 19005-2 sections 6.4.1, 6.5.1, 6.5.2; ISO 19005-4 sections 6.4.1, 6.6.1, 6.6.2, 6.6.3.

## 1. What was refused, and what is decided

Nine requirements were refused by name with one sentence saying the removal "is not built". They
are three different acts, and the reason they were one refusal is that they share one reading of
§12.6's action trees:

| rewrite | the clause | what goes |
|---|---|---|
| `ForbiddenActionRemoved` | ISO 19005-2 6.5.1, ISO 19005-4 6.6.1 | an action whose **type** the part forbids |
| `AdditionalActionsRemoved` | ISO 19005-2 6.5.2, ISO 19005-4 6.6.3 | an `/AA` **entry**, or the keys of one outside Table 197's triggers |
| `WidgetActionEntryRemoved` | both parts' 6.4.1 | a widget's or field's `/A` **entry**, with the chain behind it |

One `Loss::InteractiveBehaviour` for all nine, because a person is being asked one question:
`doc/pdf-a-conversion-limits.md` section 3.3 classes it *Ask* and gives the sentence — a form that
computed its own fields stops computing them.

## 2. The decision this ADR exists for: a removed action's `/Next` is promoted, not dropped

§12.6.2's Table 196 makes `/Next` "[t]he next action or sequence of actions that shall be performed
after the action represented by this dictionary", and NOTE 1 states the order: "[a]ctions within
each Next array are executed in order, each followed in turn by any actions specified in its Next
entry, and so on recursively."

Two constructions were available and only one is honest.

- **Drop the forbidden action and let §7.3.10 do the rest.** A reference to an object the file does
  not hold "is a reference to the null object", and §7.3.7 makes a null-valued entry the same as an
  absent one — which is how `PostScriptXObject` already works. It is one line, and it takes the
  *permitted* actions behind the forbidden one with it. A `GoTo` behind a `Launch` fails no
  requirement, and ADR 0947's second rule is that nothing changes which no failed requirement asked
  to change. So this is a loss the clauses did not ask for.
- **Promote the subtree.** The forbidden node's surviving children take its place, in order. NOTE
  1's order is preserved exactly, because the sequence *node, its subtree, then the next sibling*
  is what both readings give.

The second is built. Its one awkward shape is a position Table 166, Table 29 and Tables 197–199
each type as a **single action dictionary** with two survivors to put in it; the first survivor
takes the position and the rest go onto *its* `/Next`, which NOTE 1's own sentence says is the same
sequence. Where one action object is performed from two positions whose removals leave it two
different tails, the conversion refuses by name rather than splitting the object — writing a second
action dictionary the document does not contain is exactly what the fence forbids.

## 3. Where the population comes from

`pdf_archive` gains `action_sites`, `action_admitted`, `additional_actions_admitted`,
`action_entry_admitted` and `annotation_trigger`. **The reading of ISO 19005 stays in the
validator**, which is ADR 0947's consequence worth keeping: the converter asks *is this type
admitted at this target* and never decides it. The predicates are written over the same four static
lists the rows report from, so a converter that removed something the validator had not reported
would be a compile-time impossibility rather than a bug to find.

What the converter does read is ISO 32000-2: the trees hanging off each site are §12.6's, and
walking them is the base standard's structure rather than the archival standard's judgement.

Three readings the sites make explicit:

- **An `/AA` on an annotation that is not a widget is outside ISO 19005-2 section 6.5.2's four
  places.** The clause enumerates the catalog, a page, a widget annotation and a field dictionary;
  a `Link`'s `/AA` is none of them, and part 2 therefore leaves it. Part 4's section 6.6.3 does
  reach it, and admits only the annotation triggers.
- **An outline item states no `/AA` in either edition's tables**, so neither clause reaches one.
- **The name dictionary's `/JavaScript` goes whole at part 2.** §7.7.4's Table 32 defines the entry
  as "[a] name tree mapping name strings to document-level ECMAScript actions", so removing the key
  removes exactly what section 6.5.1 forbids — and editing the tree's leaves one at a time would
  leave a name tree whose every value the part forbade.

## 4. The fence, said once

Removing a dictionary entry invents no mark. Every content stream crosses this rewrite byte for
byte; what changes is what a reader does when a user clicks, which is not something the page shows.
`CLAUDE.md`'s fourth amendment makes provenance the test, and nothing here writes a mark of any
provenance at all. The removal is a *loss* — of behaviour — and that is why it is an authorisation
rather than a mechanical rewrite; it is not a fence question.

## 5. What the report owes

Section 3.3's condition, in the shape section 3.2's already has: an action that is gone leaves
nothing in the output to notice, so `Conversion::removed_actions` names each one — the dictionary
it was written in, the entry it was reached through (`OpenAction`, `A`, `AA /U`,
`Names /JavaScript`), the page where there is one, and what it was. The three rewrites are counted
apart, by **actions removed** rather than by objects edited: one widget may lose one entry and one
catalog six triggers, and a count of objects would tell an operator how many dictionaries changed
rather than how many buttons stopped working.

## 6. What this constrains

- **A departure from one of the three rows leaves the other two running.** Each edit carries the
  rewrite that asked for it and the rewriter applies only what a decision wanted, so the three are
  independent at the object level rather than all-or-nothing.
- **A holder written directly into its parent refuses the document**, with the same sentence
  `prepare_forbidden_annotations` gives for a direct annotation: this rewrite acts on objects.
- **A chain longer than thirty-two links, or one that reaches itself, refuses.** NOTE 1 recommends
  a processor guard against self-referential actions; a converter cannot decide which once.
