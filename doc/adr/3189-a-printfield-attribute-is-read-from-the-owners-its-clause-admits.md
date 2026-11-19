# 3189 — A `PrintField` attribute is read from the owners its clause admits

Session 2122. Status: accepted. Rows: §14.8.5.6 (`implemented`, unchanged; owner sentence applied).
Context: ADR 3177; review 2038.

## Decision

§14.8.5.6 ends: "This attribute may only be defined in attribute objects whose O (owner) entry has the
value PrintField or whose owner is any other owner excluding Layout, List, Table and Artifact".
`Tree::print_field` said this was "applied by `Tree::attribute` already", but `attribute` admits every
owner §14.8.5.3 ranks. The claim was false, so the code now does what it said:
`Tree::print_field_attribute` ranks only the admitted objects, applied to
`AttributeObject::effective_owner` as `Tree::artifact_attribute` (§14.8.5.8) and
`Tree::layout_attribute` (§14.8.5.4.1, ADR 3177) are. §14.8.5.2 makes the owner what "determines the
interpretation of the attributes defined in the object", so a `/Role` in a `List` object is not
Table 383's entry.

## Cost

Nil for a conforming file. A file that writes a field's `/Role` under `List` loses that role on a
screen reader, which is what the file said. The test's control shows §14.8.5.3 alone choosing the
`List` object's value.

## What is left

§14.8.5.5's and §14.8.5.7's parallel sentences (List, Table attributes) are not checked here; they
are the next instance of the same reading.
