# 3279 — List and table attributes are read from the owners their clauses admit

Session 2144. Status: accepted. Rows: §14.8.5.5, §14.8.5.7 (both `implemented`, unchanged).
Context: ADRs 3177 (layout), 3189 (PrintField).

## Decision

§14.8.5.5: Table 382's attributes "may only be defined in attribute objects whose O (owner) entry
has the value List or whose owner is any other owner excluding Layout, Table, PrintField and
Artifact". §14.8.5.7 states the same for Table 384, with `Table` as the owner and `List` among the
excluded. Until now the readers asked `Tree::attribute`, which admits every owner §14.8.5.3 ranks.
So a `/Scope` in a `Layout` object, or a `/ContinuedList` in a `Table` object, reached a screen
reader as a fact.

`Tree::list_attribute` and `Tree::table_attribute` apply each clause's four exclusions to
`AttributeObject::effective_owner`, as `print_field_attribute` and `artifact_attribute` do. The
ranking among the admitted objects stays §14.8.5.3's. `list_continuation` reads through the first;
`cell_span`, `header_scope`, `cell_headers`, `table_summary` and `header_short` read through the
second. `header_scope`'s comment, which said `Tree::attribute` already applied the condition, and
which put a paraphrase in quotation marks, is corrected.

## Tests

`a_list_attribute_is_read_only_from_the_owners_its_clause_admits` and
`a_table_attribute_is_read_only_from_the_owners_its_clause_admits`. Each has a control showing that
`Tree::attribute` alone takes the refused object's value.
