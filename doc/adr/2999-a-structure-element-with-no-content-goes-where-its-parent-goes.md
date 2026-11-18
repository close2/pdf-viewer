# 2999 — A structure element with no content goes where its parent goes

Session 2078. Status: **implemented.** Context: review 2038 (finding B56). Clauses: ISO 32000-2 §14.7.2 Table 355
(`/K` optional), §14.8.4.7.3 Table 369 (`Warichu`), Annex L.

The structure carry kept an element only where it reached a page the output holds, through its own content or a
descendant's. An element with no `/K` reaches no page, so a split dropped it — and Table 369's `Warichu` holds `WP`,
`WT`, `WP`, where a `WP` with no punctuation is still the boundary the sequence names, so each piece lost the
required shape. Such an element is now *contentless*: its parent keeps it, in its place among the kept children,
whenever something else keeps the parent, and blocks it with itself otherwise. A contentless element at the top of
the hierarchy is still dropped, because nothing places it with any page.

Tests: `pdf-transform` `warichu_split_spec_audit` (both).
