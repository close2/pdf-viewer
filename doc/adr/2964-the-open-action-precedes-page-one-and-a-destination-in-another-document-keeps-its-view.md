# 2964 — The open action precedes page one's, and a destination in another document keeps its view

Session 2073. Status: **implemented.** Context: review 2038 (findings B22, B23, B24, B74, D40), the owner's work
order of 2026-11-11 ("Please use a round to fix all findings"), ADR 2814 (whose section 2 this supersedes for the
moment the open action is performed), ADRs 0162, 1227, 0094. Clauses: ISO 32000-2 §7.7.2 (Table 29), §12.3.2.2
(Table 149), §12.6.3 (Tables 197, 198), §12.6.4.3 (Table 203), §12.6.4.4 (Tables 204, 205), §12.5.6.18.

## 1. Table 29's `/OpenAction` is performed at the open, before page one's `/O`

ADR 2814 section 2 performed the open action's chain at the first `Command::Presented`, after page one's Table 198
`/O` and its annotations' Table 197 `/PO` had already been raised at the open. Table 197 orders `/PO` "after … the
OpenAction entry in the document Catalog", and Table 198's `/O` sits between them. `Viewer::process` now performs
the chain immediately before page one's events; a turn the chain makes raises nothing of its own, because the page
it reaches is the page opened. Its ECMAScript actions stay the open sequence's, at the first present.

## 2. A remote or embedded destination is a page and a view

Table 203's and Table 204's `/D` is a destination, and Table 149 makes a destination a page *and* the window's view
of it. `interact::place_remote` and `jump_into` kept the page and dropped the view; both now leave it in the
replacement's `pending_views`. A `/GoToR` whose page came from `/SD`'s structure element keeps no view of `/D`'s.

## 3. `/FitR` and `/FitB` centre the box in the dimension the smaller factor leaves

Table 149: "use the smaller of the two, centring the rectangle within the window in the other dimension", and the
same of `/FitB`'s bounding box. `apply_view` put the box's corner at the window's corner. The box is now carried
into the display list's space (a `/FitR` rectangle through the page's transform, so a rotated page's horizontal is
the window's) and the spare room is split on both sides (`centred_corner`). Errata Collection 3's issue 536 reads
the `/FitR` sentence as a recommendation; this reader follows it.

## 4. Table 205's `/R /P` reaches the document a child came out of

ADR 0094 walked the path from the document showing with no parent above it, so a child opened by an embedded go-to
could not name its parent, and a walk `C` then `P` that ended where it began answered "no parent".
`EmbeddedGoTo::target_in` now walks a *lineage* — the source and the documents it was opened out of, root first —
and answers the target's lineage; `Open::parents` keeps it. A walk ending in the document showing is a jump within
it, and one ending in a parent shows that same document again. The cost is the parents' bytes held while a child
is shown, which is what the clause's "parent of the current document" needs.

## 5. §12.5.6.18's `/P` bullet is executed, not vacated

The ledger row said the bullet bound an out-of-scope action. The rendition action is in scope (A254, ADR 1960)
and `Rendition::unplaced` checks the bullet before anything of the action is done; the row now says so.
