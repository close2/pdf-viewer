# 1381 — A tagged page's widget that no element names is still a control

Session 1272. Status: **accepted** and built.
Context: `crates/viewer-core/src/viewer.rs` (`accessibility`, `structure`, `unreached_widgets`),
`crates/viewer-core/src/query.rs` (`PageStructure::widgets`), `crates/viewer-accessibility/src/tree.rs`
(`elements`, `widget_nodes`), `crates/viewer-core/tests/{untagged_widgets,accessibility_census}.rs`.
Amends: ADR 1369 section 2 ("filled only where the document states no `/StructTreeRoot`").
Clauses: ISO 32000-2 §12.5.1, §12.5.3, §14.7.5.3, §14.7.5.4, §14.8.2.2.1, Table 358, Table 368,
Table 226, Table 31.

## 1. The reading

The brief named Table 368 for `/OBJR`. That is wrong: the object reference dictionary is **Table
358** (§14.7.5.3), and Table 368 is §14.8.4.7.2's inline types, whose `Form` row says "[i]n a tagged
PDF, Form shall be used for each PDF widget annotation that belongs to the real content of the
document". That `shall` binds the producer. §14.8.2.2.1 says the logical structure "encompasses all
real content", and real content is defined by the author's intent ("material intentionally
introduced by the document's author and necessary to understand the content"), not by whether it was
tagged; an artifact "should be explicitly distinguished" (§14.8.2.2.2), which an omission is not. So a
widget a tagged file leaves out of its structure is a producer's omission, and nothing in §14.7 or
§14.8 lets a reader withhold what the page lets a person do: §12.5.1's click is one definition for the
mouse and for the bus (ADR 0425).

## 2. Decision

- `PageStructure::widgets` holds, on every page, the interactable widgets no published element names.
  "Names" is measured on what was published: every object any gathered element's own content items
  name (§14.7.5.3's `/Obj`, Table 357's `/StmOwn`), so a widget is never both an element and in the
  list, and a widget the parent-tree pruning or the node bound left unreached is offered.
- On a tagged page they come **after** the structure's own nodes, in §12.5.1's tab order (Table 31's
  `/Tabs`), named by Table 226's `/TU`, with ADR 1369's band. No place in the structure's reading
  order is invented for them.
- A tagged page not yet interpreted answers neither half; an untagged one still answers its widgets.
- The census counts tagged pages publishing such widgets and how many, and holds at zero the class of
  a widget published both as an element and in the list (replacing ADR 1369's "a tagged page with a
  list" class, whose premise this decision retires).

## 3. What it leaves

A tagged page whose structure reaches nothing on it publishes ADR 0214's untagged sentence, which
is false of the document; `PageView` does not know whether the document is tagged.
