# 2028 — A host speaks what the tree resolved, a `Private` element is not presented, and a circular role map stops where it returns

Session 1596, slot 5 of batch ninety-four, a clause round. Status: **accepted** and **built** in
`crates/pdf-model/src/structure.rs` (`Tree::mapped_role`, `MappedRole`, `Nonstandard`,
`Tree::walk_role_map`), `crates/viewer-core/src/accessibility.rs` (`AccessibilityNode::standard`,
`walk`'s `Private` arm, `withhold`), `crates/viewer-accessibility/src/role.rs` and `tree.rs`, and the
confined wire (`protocol/panels.rs`, greeting `PDFVCF17`). Context: ADR 2014 section 5, which left
the first two of these as unfinished; ADRs 0214, 0785, 1381. Tests: `structure.rs`'s
`a_circular_role_map_stops_at_the_first_type_it_returns_to` and `a_type_that_is_not_standard_says_why`;
`headless.rs`'s `a_private_element_and_its_descendants_are_not_presented`; `viewer-accessibility`'s
`a_homonym_is_what_the_tree_resolved_and_not_what_its_name_reads_as` and
`a_homonym_crosses_as_what_the_tree_resolved`; the confined protocol's accessibility round trip.

## 1. A host is handed the reading, not only the name

`AccessibilityNode::role` crossed as a string (ADR 0214), and `viewer-accessibility` read it again
with `StandardType::read`. §14.8.4's vocabulary is defined by §14.8.6.1's two namespaces, so the
name alone cannot say whether `Table` is §14.8.4's table or a foreign vocabulary's homonym, and
§14.8.6.2's "[a]n element shall be considered to be in one of these namespaces if:" is answered only
where the namespace is in hand. `Tree::standard_role` refused the homonym; the host then mapped it
as a table.

**Built**: `Tree::mapped_role` answers the name and its meaning in one walk — the standard type, or
one of four reasons there is none (`Nonstandard`: no `/S`; a name its standard namespace does not
define; a namespace that is not standard; a circular role map). `AccessibilityNode::standard`
carries it beside the name, `viewer_accessibility::role::map` takes it and reads the name for the
description only, and the description says which reason. The name still crosses, because §14.8.4.1
makes a non-standard name a fact about the document a host may say. On the confined wire the
reading is one byte after the name: a standard type is read back from the name, since
`mapped_role` answers one only for a name that reads as exactly that type, and the refusal — which
the name cannot carry — is the byte. The format changed, so the greeting moves to `PDFVCF17`.

## 2. `Private`, element and descendants, is not presented

Annex M lists `Private` among the eleven types only the PDF 1.7 namespace defines, and §14.8.6.1 names
ISO 32000-1's clause 14.8 as that namespace's schema; its Table 333 says neither the `Private`
element nor any of its descendants shall be interpreted or exported to other document formats.
ADR 2014 mapped the element to a filtered, unspoken container, and the platform filter then lifted
its children into the tree — the descendants were presented.

**Built**: `viewer-core`'s walk gathers neither the element nor anything below it: no node, no text,
and no content item pushed to an ancestor's place. The subtree is still *reached* (`withhold`),
because §14.7.5.4's parent tree names its elements for the page, and a walk that had not reached
them would read as one whose two chains disagree and be run again over the whole document. A widget
a withheld element's object reference names is then named by no element and is offered as one the
structure left out — ADR 1381's rule, for its reason: §12.5.1's interactive content is not the
structure's to withhold, and a person must still reach the field.

Not built, and said: `Tree::logical_range`, the copy in logical order, still copies a `Private`
element's text. Whether a copy to the clipboard is an export "to other document formats" is a
reading this round did not settle, and `structure.rs` was this round's for the role-map stop only.

## 3. A circular role map stops at the first type it returns to

§14.7.3's NOTE 2 permits the circle and says where to stop: "A PDF processor using the role map needs
to follow the chain of associations until it either finds a structure type it recognises or returns
to one it has already encountered." The walk ran `MAX_DEPTH` laps instead and answered whichever
name the lap count's parity left, so the NOTE's own bidirectional map `MyPara` ↔ `P` read as `P` from
one end and `MyPara` from the other.

**The normative sentence decides an acyclic chain and the NOTE a circular one.** "A structure type
shall always be mapped to its corresponding name in the role map, if there is one, even if the
original name is one of the standard types", so a chain through a standard name the map maps onward
is followed past it, as before. A circle has no name the `shall` can end at, so the NOTE's two stops
are what is left: the walk ends at the first (name, namespace) pair it returns to, and answers with
the first type along the circle — the element's own name excepted, which the `shall` maps at least
once — that a standard namespace defines; the NOTE's map then reads as `P` from either end. A circle
holding no standard type answers its last new name with `Nonstandard::Circular`, which is said once,
in the node's description, rather than looped over silently. `MAX_DEPTH` still bounds a chain of
distinct names.

## 4. What it moved

`accessibility_census`, HEAD's export and this round's: see `doc/history/1596`. No drawing reads a
structure type, so no page digest can move; the six arms are the check.
