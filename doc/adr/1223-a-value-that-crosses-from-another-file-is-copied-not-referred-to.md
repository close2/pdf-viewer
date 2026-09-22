# 1223 — A value that crosses from another file is copied, not referred to

Status: **accepted**.
Context: `crates/pdf-model/src/forms_data.rs` (`carry`, `carry_dictionary`, `Carried`,
`FdfField::appearance`, `FdfField::actions`, `Import`), `crates/pdf-model/src/view.rs`
(`AnnotationView::imported_appearance`, `ViewState::imported_actions`,
`ViewState::write_imported_appearances`, `promote_streams`, `write_added_appearance`),
`crates/pdf-model/src/annotation.rs` (`stored_appearance`),
`crates/viewer-core/src/interact.rs` (`trigger`).
Builds: ADR 1186 (what crosses from another file, and what only names one), ADR 0907, ADR 1155,
ADR 1185, ADR 0100 (an edit is a log beside an immutable document), ADR 1062.
Amends: nothing; it answers the design question ADR 1186 left open at two sites.
Clauses: ISO 32000-2 §12.7.8.3.2 (Tables 249 and 250), §12.7.8.3.4 (Table 254), §12.5.5
(Table 170), §12.6.3 (Table 197), §7.3.8.1, §7.3.10, §7.5.6.

## The question ADR 1186 left open

Three of Table 249's entries, and the whole of Table 254's annotation dictionaries, were unapplied
for one reason with two descriptions. The ledger's was "drawing one means a second `Document`
reaching the interpreter"; `forms_data.rs`'s was "an action read out of an FDF file resolves its
own references *there* while this tree reads a widget's from the target document". Both are the
same fact: the **value** of the entry is an indirect reference into a file that is not the one
being drawn.

The obvious design is the one both descriptions imply — give the interpreter a second `Document`
and a rule for which one a reference resolves in. It is also the wrong one, and `CLAUDE.md` says
why in a sentence written for a different purpose: `interpret` is a pure function of what the file
says, what the viewer state is, and what the user did, and the oracle's whole comparison rests on
the first of those being a function of *the bytes*. A second object space reachable from inside
interpretation makes "the bytes" ambiguous at every `resolve`.

## The decision

**Nothing crosses as a reference. Values cross as values.**

§12.7.8.3.2's binding sentence is about values and says so: "importing a field causes the values
of the entries in the FDF field dictionary to replace those of the corresponding entries in the
field with the same fully qualified name in the target document." A value that is an indirect
reference into another file is not a value this document can hold. So `forms_data::carry` resolves
every reference and copies what it finds in its place, recursively, and what comes back is a tree
of **direct** objects that names nothing of the other file.

After that there is no second document to have a rule about. `pdf_syntax::Document` stays immutable
and singular; the copy lives in the log beside it, which is where every other thing that is not the
producer's bytes already lives (ADR 0100).

Three entries are applied on it:

- **`/AP`.** Table 249's appearance dictionary, whose `/N`, `/R` and `/D` "shall all be streams",
  crosses with its `/Resources` and everything under them. `AnnotationView::imported_appearance`
  carries it and `annotation::stored_appearance` reads it *instead of* the widget's `/AP` — which
  is the replacing sentence applied to Table 170's entry, since Table 249 says its own `/AP` is
  "as shown in "Table 170 - Entries in an appearance dictionary"".
- **`/A` and `/AA`.** They cross into one dictionary rather than two fields, because §12.6.3's
  Table 197 states a precedence *between* them: "For backward compatibility, the A entry in an
  annotation dictionary, if present, takes precedence over this entry." `viewer_core::interact`
  composes the carried entries over the widget's own dictionary and asks `action::for_annotation`
  once, so the rule is read where it already was and not a second time.

## Why the copy is refused whole rather than truncated

`Carried` bounds the references followed and the stream bytes taken, per entry that crosses. When
either runs out, `carry` answers `None` and the *whole* entry is refused and named on
`FdfField::owed` — half an appearance is a button drawn wrong, and trap 5's rule is that a partly
implemented feature is where a silent fallback hides best. The same bound is what makes a `/Next`
chain that loops terminate: each hop spends a reference.

The numbers are bounds on **work** and say so. The standard states none here and nothing it
requires a reader to carry states one either (trap 38), which is the condition under which a
number in this tree has to be justified as a budget rather than as a limit.

## Writing it back, and why that is not authoring

A save writes the imported `/AP` into §7.5.6's update as the widget's appearance. `CLAUDE.md`'s
boundary asks one question — **does the operation invent marks?** — and every byte of a carried
stream was written by whoever produced the FDF file, carried across unreinterpreted and placed
under the key that file named. Nothing here composes anything, so this is on the near side of the
line for the same reason ADR 1120's relocation is: the provenance of the marks is a producer's.

`promote_streams` is where the copies become objects. A carried appearance holds its streams
*directly*, because a direct object names nothing and that is the point; §7.3.8.1 requires a stream
in a file to be an indirect object, so each is given a number in the update as it is written, and
the references take their place. `write_added_appearance` gains the same guard from the other
direction: an annotation that arrived with an appearance of its own keeps it, because constructing
over it would replace another producer's marks with this program's.

## What this does not answer

`/APRef` still has no reader, and this round narrowed rather than closed it — see ADR 1224 and
§12.7.8.3.2's ledger row. `/RV` is XFA rich text, on `CLAUDE.md`'s closed exclusion list.
