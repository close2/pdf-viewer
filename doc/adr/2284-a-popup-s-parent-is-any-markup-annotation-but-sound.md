# 2284 — A popup's parent is any markup annotation but Sound

Session 1725, slot 6 of batch one hundred and eleven, a clause round. Status: **implemented**.
Context: ISO 32000-2 §12.5.6.2, §12.5.6.6, §12.5.6.14, §12.5.6.24, Tables 171 and 172; the archive's
linked-popup removal (`crates/pdf-transform/src/archive/in_place/linked_popups.rs`), whose list of
parent subtypes held `FreeText` and `Redact`, which §12.5.6.2's list of annotations with a popup
window does not name. It moves no ledger row. ADR 2285 is unused.

## 1. The question

The removal takes a hidden markup annotation and its hidden popup off a page together, and only a
closed reciprocal pair: Table 172's `/Popup` one way, Table 186's `/Parent` the other. It asks the
parent's subtype first, and two lists in the standard could be the answer. §12.5.6.2's second group
reads "Most other markup annotations have an associated popup window that may contain text … These
include text, line, square, circle, polygon, polyline, highlight, underline, squigglyunderline,
strikeout, rubber stamp, caret, ink, and file attachment annotations." Table 171's Markup column
says `Yes` of those, and of `FreeText`, `Sound`, `Redact` and `Projection` too.

## 2. Decision

**The list is Table 171's Markup column less Sound**, and `Projection` joins it. The reasons are
the sentences that define the relationship the removal reads:

- §12.5.6.14: a popup annotation "shall not appear alone but is associated with a markup
  annotation, its parent annotation, and shall be used for editing the parent's text", identified
  "by the Popup entry in the parent's annotation dictionary". The parent is *a markup annotation*.
- §12.5.6.2: Table 172 "lists annotation dictionary entries that apply to all markup annotations",
  and `/Popup` is one of them.
- §12.5.6.24 gives a projection annotation the entries of Table 166 and of Table 172 and no
  others, so it holds `/Popup` by name, and the list lacked it.

§12.5.6.2's groups describe what a reader shows and are not the definition: `Redact` is in none of
the four, so they do not partition the Markup column, and the second group's list opens with "These
include". **Sound is the one exclusion**, because §12.5.6.2 says of it what it says of no other
subtype: "Sound annotations do not have a popup window". `FreeText` stays in. §12.5.6.6 says a free
text annotation's text is displayed "instead of being displayed in a popup window", which says
where its text appears. It does not say the annotation has no popup, and Table 172's `/Popup` still
applies to it.

## 3. What it changes

A closed hidden pair whose parent is a `Projection` annotation is removed where it was refused.
Every other subtype's answer is unchanged. `a_popup_pairs_with_every_markup_subtype_but_sound`
(`crates/pdf-transform/tests/archive/linked_popups.rs`) holds the list. With `Projection` taken out
of it again, that test fails. `FileAttachment` is markup, but the test leaves it out, because ISO
19005-4 section 6.3.1's own site answers it before the pair proof is asked.
