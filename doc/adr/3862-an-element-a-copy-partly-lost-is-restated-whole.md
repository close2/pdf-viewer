# 3862 — An element a page's copy partly lost is restated whole, though another page shows it

Session 2290. Status: **accepted**, the alternative asked as `doc/questions/Q1111-split-an-element-a-redacted-copy-partly-lost.md`. Context: ADRs
3725, 3809, 3811, 3849; ISO 32000-2 §12.5.6.23, §14.7.2 (Table 355), §14.9.3, §14.9.4.

## Context

A structure element whose marked-content sequence lies in a form two pages draw, redacted on one,
is reached in that page's copy (ADR 3849). ADR 3725 then restates it without `/Alt`,
`/ActualText` and `/E`, though the other page still draws the producer's form and the whole
sequence. The brief asked whether restating should follow the copy alone.

## The reading

- §14.9.3's alternate description and §14.9.4's replacement text describe or replace the
  element's content **as one**; nothing in Table 355 attaches either to one of an element's content
  items. ADR 3810 read the same for a part of the codes.
- After the redaction the element names the copy too (ADR 3811): its `/K` reaches the copy's
  sequence, where the covered text is gone. Kept, the description is reached from the redacted page
  and states the removed text — the trace §12.5.6.23 forbids.
- Following the copy alone therefore means **two** elements: the producer's, naming the form, with
  its descriptions; and a new one naming the copy, without them. §14.7.2 allows it (an element is
  any structure dictionary), and it writes structure, not marks. But it changes the logical
  structure the producer wrote — one unit becomes two siblings — which ADR 1363 refused to do to a
  form's own structure, and it needs a new parent-tree entry for the copy's sequence (ADR 3849)
  to name the new element instead.

## Decision

The element stays one and is restated whole. The cost is a description lost for the placement the
user did not redact: no mark is lost, the report names the restating, and §12.5.6.23's trace is
gone. Since ADR 3861 a copy whose original nothing else draws takes the original's place, so the
case arises only where another page really draws the form. Splitting the element is asked as
`doc/questions/Q1111-split-an-element-a-redacted-copy-partly-lost.md`. `redact::an_element_a_copy_lost_is_restated_though_another_page_draws_the_form_whole`
holds the decision.
