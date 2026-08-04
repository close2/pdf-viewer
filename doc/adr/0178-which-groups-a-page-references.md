# ADR 0178 — Which groups a page references, and who gets to say

Status: accepted, two-hundred-and-fifty-fourth session.

## Context

The previous session's finding — a clause implemented in `pdf-model` that no host called — is
worth a sweep of its own, so this one ran it: every `pub fn` in `pdf-model`, grepped against
`viewer-core` and `viewer-ui`. 174 functions, 72 that neither host-side crate names. Most are
internal helpers that happen to be `pub`. One was a clause with a switch on the screen already:

```
list_mode
```

`OptionalContent::list_mode` reads Table 99's entry, `Query::Layers` presents `/Order` unfiltered,
and the ledger explained the gap with

> `/ListMode`'s `VisiblePages` is carried and not acted on, because which pages are visible is a
> question about a window this crate does not have

— a reason about what the *program* lacks, eighty-six sessions after the window arrived and
eighty-seven after the layer panel it feeds.

## What the clause says

§8.11.4.3, Table 99:

> A name specifying which optional content groups in the Order array shall be displayed to the
> user. Valid values shall be: AllPages Display all groups in the Order array. VisiblePages
> Display only those groups in the Order array that are referenced by one or more visible pages.

Two questions, and they belong to two crates.

## Decision

**`pdf-model` answers "which groups does *this page* reference"; `viewer-core` answers "which
pages are visible".** `optional_content::groups_referenced_by(document, page)` is the first;
`viewer::layers` intersects it with the page being shown.

The window's half is a derivation this tree has already made once. A window showing one page at a
time makes "one or more visible pages" the page it is showing — the same reading the
two-hundred-and-fourth session took for §12.6.3's `/PV` and `/PO`, where the clause's own reason
for distinguishing them is that "[a]t any one time, while more than one page may be visible,
depending on the page layout".

The file's half is a **reading**, because the clause does not say what *referenced by* means. What
is taken is the three places §8.11 puts an `/OC`: the page's `/Resources /Properties`, which is
what a `BDC /OC` names (§8.11.3.2); an `XObject`'s own entry (§8.11.3.3); and an annotation's,
Table 166's (§8.11.4.4). A membership dictionary contributes every group its `/OCGs` or its `/VE`
mentions, since content governed by one is content those groups decide the visibility of. Nested
form `XObject`s' resources are followed, because a group referenced by a template placed on the
page is referenced by the page — bounded by a visited set and by a depth of eight, both because
the nesting is a document's word.

**The walk deliberately does not interpret the page.** A `BDC /OC` naming a property this walk
found makes the group reachable; whether that operator actually runs is a different question, and
one the panel is not asking. The direction of the error matters and is stated as such:
over-listing a group whose `BDC` never executes costs a person a switch that does nothing;
under-listing one hides a switch the document asked to show.

## What the corpus says, which is almost nothing

**One document states `/ListMode`** — `visibility_expressions.pdf`, on a scan of every
uncompressed occurrence in all 974 — and it states `VisiblePages`. Its single page reaches all
three of its groups through the `/VE` of five membership dictionaries, so the filter changes
nothing there. That is what the headless test pins, and it pins the direction that costs
something: **a filter that is applied must not empty a panel the document meant to fill.** The
filtering case itself has no witness and is a synthetic fixture in
`pdf-model/tests/optional_content.rs`, with five groups of which one is named by nothing on the
page.

## The lesson

**"Who calls it?" is the sweep this project did not have.** `doc/todo/01`'s three sweeps all ask
what a *row* claims. This one asks what the *code* offers and nobody takes, and it found a clause
in its first run — as the previous session's did, from the other direction. Both findings are the
same shape: a capability arrives, and the rows and the callers of the code it unblocks are
maintained by nobody, because neither cites the other.
