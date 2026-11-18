# 2978 — A non-interactive form field reaches an accessibility host as its Table 383 attributes

Session 2075. Status: **implemented.** Context: review 2038 (finding B29), ADR 1369 (interactive widgets as
nodes). Clause: ISO 32000-2 §14.8.5.6.

`Tree::print_field` read Table 383's `/Role`, `/Checked` and `/Desc` and nothing used them, so a flattened check box
reached a screen reader as an unnamed `Form` whose state was lost. `AccessibilityNode::print_field` now carries
them for a `Form` element (the clause's "Attributes governing Form structure elements for non-interactive form
fields"), and the confined protocol encodes them beside the artifact attributes.

They cross as the attributes, not as a `Control`: `Control` is an interactive widget a host may act on, and §14.8.5.6
is about forms that "were then converted into non-interactive PDF files, or … designed to be printed out and
filled in manually". A host maps the role and state to its platform's read-only form roles and the description to
the accessible description; nothing offers an action the document does not have. The node's name is unchanged.
