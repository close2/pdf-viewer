# 1339 — An imported page is a group under its own page `/Group`, inside the proxy's

Status: accepted and **built**. Session 1251.
Amends: ADR 1101 (which drew the imported page into the containing page's compositing).
Depends on: ADR 0237, ADR 1101, ADR 1239.
Context: `crates/pdf-model/src/content/xobject.rs` (`ImportedPage`, `run_group_body`,
`run_imported_page`), `crates/pdf-model/src/content/transparency.rs` (`GroupBody`,
`imported_page_group`), `crates/pdf-model/tests/reference_xobjects.rs`.
Clauses: ISO 32000-2 §8.10.4.1, §11.4.1, §11.4.4, §11.4.5, §11.4.7, Table 145.

`§N` is ISO 32000-2 and nothing else.

## What the clause says, and what the brief said

§11.4.7 treats a page in one of two ways. The second is the one a reference `XObject` reaches:

> In this situation the PDF 'page' shall not be composited with the media colour; instead it shall
> be treated as a transparency group using the page Group attributes dictionary and is composited
> with its backdrop in the usual way according to the page Group attributes dictionary settings.

The dictionary is the **imported** page's own. The brief had it the other way round — "under the
containing page's group attributes instead of its own" — and the ledger row's own note had it right;
the containing page's group is only the parent the imported group is composited into.

§8.10.4.1 adds a second dictionary for the same page:

> If the proxy object's form dictionary contains a Group entry, the specified group attributes
> shall apply to the imported page as well, which allows the imported page to be treated as a group
> without further modification.

## Decision

1. **The imported page is always a group.** Its attributes are its page `/Group`'s, or Table 145's
   defaults (not isolated, not knockout, space inherited) where it states none. §11.4.4's NOTE 5
   then draws a default group inline wherever its `Do` composites trivially — the old picture — and
   as one object under a constant alpha, a mask or a blend mode at the proxy's `Do`, where the old
   route applied them mark by mark.
2. **Two dictionaries, nested.** Neither clause says one replaces the other, and nesting obeys
   both: the proxy's group holds one element, the page's group. Where the proxy's group is a
   knockout group this is visible — the page's marks composite with each other inside it rather
   than knocking each other out.
3. **Each dictionary is read in its own document.** The proxy's `/Group` (and any `/CS` it names)
   is the containing file's object, so it is read and its group entered before the target document
   is swapped in; the page's is read after. The old route read the proxy's `/Group` after the swap,
   which resolved an indirect `/Group` in the wrong file — trap 1's shape, and a fixture now holds it.
4. **The page's annotations are elements of the page group**, since §8.10.4.3 makes them part of
   "the rendering of the imported page".

A group's content is `GroupBody` rather than a content stream, because a group may run its content
more than once (its own space, then given up; §11.4.7's pair) and an imported page is two scopes
entered in document order.

## Evidence

No document on this disk states a reference `XObject` (§8.10.4's census), so four fixtures are the
evidence, each calibrated by planting the old route back: an isolated page group against Multiply
over green (red, not black); `ca 0.5` at the proxy's `Do` taken once (`(127, 127, 255)` at the
overlap, not `(127, 63, 191)`); a knockout proxy group whose one element is the page (black, not
blue); and a proxy `/Group 6 0 R` found in the containing document.
