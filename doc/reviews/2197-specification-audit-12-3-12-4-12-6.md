# Specification audit of ISO 32000-2 §12.3, §12.4 and §12.6 (navigation and actions) — round 2197

The project's thirteenth specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2195 ([`2195-specification-audit-8-9-8-11.md`](2195-specification-audit-8-9-8-11.md)).
Baseline: main at `66ec8652`. Evidence:
[`2197-specification-audit-12-3-12-4-12-6/`](2197-specification-audit-12-3-12-4-12-6/) —
`findings.json` (2038's schema) and `expected-failures.json` (the failing tests of each behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §12.3, §12.4 and §12.6 (Tables 149 to 223) was extracted from
   `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 10544 to 11092 and 11853 to 12392, and set against
   the code that reads the entry or performs the step. Most are constraints on the file; those
   were read for what a reader does when a file breaks them.
2. 2038's findings on these clauses were read first and are not repeated: B05 and B06 (the
   collection's details view) and D01 (§12.3.5.3's renumbering).
3. The readers traced: `pdf-model`'s `destination.rs` (Table 149, §12.3.2.3's algorithm,
   §12.3.2.4's two tables), `outline.rs`, `page_label.rs`, `article.rs`, `navigation.rs`
   (Table 164, navigation nodes), `collection.rs`'s initial document and sort, `thumbnail.rs`,
   `action.rs` (every type `one` reads, the `/Next` walk, Table 197's precedence of `/A` over
   `/U`) and `view/actions.rs`; `viewer-core`'s `open/destinations.rs` (the view applied),
   `interact.rs` (activation, `resolve`, the remote and embedded go-tos), `presentation.rs`,
   `transition.rs` and `viewer/navigation.rs` (the page triggers' order, `/Dur`); and
   `viewer-host`'s outline rows and `may_open_uri`.
4. The lens was the earlier audits' habits: list every route to a thing (a go-to action is read
   from a link, an `/OpenAction`, a named destination's dictionary and an outline item, and a
   structure destination is resolved locally and in a remote file); a rule keyed on a name
   applies wherever the name is read (Table 202's `/SD` and Table 203's); a bound has two sides
   (`/FitR`'s rectangle written either way round).
5. Each B finding has a failing `*_spec_audit` test whose control passes on main:
   `pdf-model/tests/navigation_12_3_12_4_12_6_spec_audit.rs` (a module of `navigation_suite`),
   `viewer-core/tests/navigation_12_3_12_4_12_6_spec_audit.rs` and
   `viewer-host/tests/outline_item_style_spec_audit.rs`.

What holds, traced or probed, is not listed one by one. Among it: `/XYZ`'s null `left`, `top` and
`zoom` (and a zero zoom) keeping the window's current values, read in user space before the
magnification changes; `/FitR` with its corners written in either order, and a degenerate side
fitting the other; `/FitB`'s bounding box clipped to the page; a named destination looked up in
the table its key's type is paired with first and in the other second (ADR 2985), with a cycle
between names bounded; `/SD` preferred to `/D` on a link's go-to, the `/OpenAction`'s and a
`/Dests` dictionary value, an element found through its kids in array order, a dangling
reference answering nothing and a structure element without content answering the first page;
the outline's `/First`–`/Next` walk under a visited set, `/Count`'s sign as the open state;
page labels' five styles, `/St`'s floor of 1, a label of the prefix alone, ranges sorted and a
range starting past the last page labelling nothing; a thread named by reference, index or
title (the first of a repeated title); Table 164's defaults and `/R`'s ignored `/D`, the
arriving page's `/Trans`; `/PresSteps` forward and backward and on arrival; `/Next` performed in
order inside the model and a cycle cut; `/A` taking precedence over `/AA /U`; `/PO` after `/O`
and `/PC` before `/C`, and the four page-scoped annotation events fired whatever the flags;
Named actions' four names and an unknown name doing nothing; `/SetOCGState` left to right,
`Toggle`, `/PreserveRB`'s default and its one-sided rule; Hide's `/H` default and its two target
forms; URI `/IsMap` rounded from the upper-left corner and ignored off a click, `/Base` and a
relative reference resolved against the document's location by the host; `/NewWindow` carried,
and ignored for a launched file that is not a PDF; Launch's `/Win`, `/Mac` and `/Unix` declined
by name.

Readings left, each a question about the standard or about the file rather than a defect found:
Table 209's `/D` index counts `/Threads` entries and `Articles::read` drops an entry that is not
an indirect reference to a dictionary, which shifts every later index — but Table 163 makes a
thread with beads an indirect object, so only a malformed array reaches it; `/St` and the label
styles have no bound, and `/St 4000000000 /S /A` spells a label of millions of letters per page
(a resource budget, principle 3, rather than a clause); and when §12.6.4.15's transition action
is played inside a chain — "after completion of the previous action" — was not traced, and is
the first thing B03's fix round reads, since the two share `resolve`.

## Findings

Four behaviour gaps. In order of how much a person would see:

- **2197-B03** — a `/Next` chain moves the view once. `interact::resolve` keeps the first page
  any request names and computes `NextPage` from the page the chain started on, so two
  `NextPage` actions advance one page and a go-to followed by another stays at the first; §12.6.2
  says the actions "shall be performed in order".
- **2197-B04** — no host draws an outline item's `/C` or Table 152's italic and bold. `pdf-model`
  reads all three and `viewer_host::outline_rows`, which every toolkit's outline panel is built
  from, carries none of them.
- **2197-B02** — a remote go-to's structure destination decides the page and nothing else: the
  rest of `/SD`'s array (its Table 149 view) is dropped, and an element found without content
  hands the page back to `/D` where §12.3.2.3 takes the first page.
- **2197-B01** — an outline item whose `/A` is a go-to with `/SD` resolves to `/D`'s page:
  `outline::destination` is the one route to a go-to action that does not go through
  `Destination::of_go_to`. Activation is unaffected; the section a page is announced under is not.

Two documentation findings: D01, `outline::destination`'s doc says nothing this program does
could follow an action other than a go-to, which `Command::Activate` does for every type; D02,
`Jump`'s doc and `resolve`'s comment say a chain that jumps twice "has shown the second page
either way and §12.6.2 states no rule for the pair" — the code shows the first page, and the
rule is the order (B03's cause).

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `12.3.3` (B01, B04,
D01 — its note lists `/C` and the flags as read and "a destination from `/Dest` or from a go-to
action's `/D`"), `12.6.4.2` (B01), `12.3.2.3` (B02), `12.6.4.3` (B02), `12.6.2` (B03, D02),
`12.6.4.12` (B03).

## Suggested fix groups

1. **`pdf-model/src/outline.rs`'s `destination`** — B01 and D01: the go-to read through
   `Destination::of_go_to`, `Item::destination`'s doc and the function's restated.
2. **`pdf-model/src/action.rs`'s `remote_go_to` and `RemoteGoTo::page_in`, with
   `viewer-core/src/interact.rs`'s `place_remote`** — B02: `/SD`'s view carried beside the
   element's identifier and applied on the page it identified, and page 0 for an element found
   without content.
3. **`viewer-core/src/interact.rs`'s `resolve` and `Jump`** — B03 and D02: the page-naming
   requests folded in order, a named action computed from the page the previous one reached.
4. **`viewer-host/src/panel.rs`'s `PanelRow` and `row_of_item`, and each toolkit's outline tree**
   — B04: the colour and the two style bits carried and drawn.
