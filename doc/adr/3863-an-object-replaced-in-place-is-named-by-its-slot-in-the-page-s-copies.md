# 3863 — An object replaced in place is named by its slot where the page's resources are rebuilt

Session 2290. Status: **accepted**. Context: ADR 1196 §3 (`privatise`), ADR 3861;
ISO 32000-2 §12.5.6.23, §8.10.1. Code: `crates/pdf-transform/src/redact.rs` (`reserve_slots`).

## Context

`privatise` rebuilds every object on the path to one of the page's copies and carries the rest by
reference. An object the page **replaces** — a form it owns — is not in that map, so where it
holds a nested form another page draws (copied for this page), `privatise` found it reaching a
copy and rebuilt it **from the producer's object**: the page's resources named a new object
holding the form's original stream, covered marks included, beside the edited one in the form's
slot. `redact::an_owned_form_holding_a_form_another_page_draws_is_not_rebuilt_from_the_producer_s`
found the outer form's `HIDDEN` in the output without this change. ADR 3861 makes the shape
common: an inherited form taking its edit in place holds the same nesting.

## Decision

Where a page has copies, every object it replaces in place enters the same map under its own
slot, so `privatise` names the slot and stops there. The slot holds the edited object, built with
its own resources carried the same way, which is the object the page's names were always meant to
reach. A page with no copies is untouched: its resources are carried by reference as before.
