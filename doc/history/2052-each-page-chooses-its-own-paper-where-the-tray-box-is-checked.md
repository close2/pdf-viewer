# 2052 — Each page chooses its own paper where the tray box is checked

Session 2052, a viewer-host round in the audit worktree (`doc/todo/76` row 6; §12.2, Table 147's
`/PickTrayByPDFSize`). ADR 2919; no question.

**The slice, and why.** Row 6 is `doc/todo/65` section 1's host rows: the per-host reads of print and collection
entries. A census of Table 147's print half and §12.3.5 across the four windows found one entry the library read
and no dialogue presented: the tray box. It is built in the two windows that print; the WinUI half is named.

**The premise.** Section 1 names no row of its own, at row 6's creation as now; §12.3.5's collection is presented
in all four windows; §12.2 is `departed` for the owner's `/HideMenubar` alone. `quorra-gtk` declined the tray
entry in a comment of an earlier round (no owner word), `quorra-qt` said nothing. Effort only; no question.

**Built.** `viewer-core`: `Query::PrintSize`, the page's print extent from its dictionary, `None` outside a print
operation; carried by `viewer-confined` (refused unless finite and positive) and `viewer-ffi`'s
`quorra_print_size` (header, C# interop, counts 379 → 380 and 360 → 361). `viewer-host`: `tray_by_page_size`,
`TRAY_BY_PAGE_SIZE`, `paper_for_page`. `viewer-gtk`: the one custom tab now carries the box and ADR 2908's hold;
checked, `request-page-setup` sets each page's paper (a named size within 1 mm) and states the sheet first.
`viewer-qt`: the box beside scaling; checked, each page's page layout (`FuzzyMatch`) and sheet, painted whole.

**Tests.** Core: the size before the grant, during it (400 and 360 by `/PrintArea`), equal to the list's, none
past the pages. Host: the box's three states; landscape paper. GTK: the box opens and is read back each way; A4
landscape and a custom paper. Confined: the round trip and four impossible sizes. C: `--enforced` reads 200 by
100, page 2 and after the job answer no. **Plants**: the box checked on any stated value, and the size answered
outside a job: the host and core tests failed; reversed, pass. Found on the way: GTK's `create-custom-widget`
return is borrowed, so a tab nothing held was finalised and no tab appeared; the handler now keeps it.

**Gates.** Over `viewer-core`, `viewer-confined`, `viewer-host`, `viewer-ffi`, `viewer-gtk`, `viewer-qt`,
`viewer-ui`: fmt 0; clippy `-D warnings --all-targets` 0; nextest under `xvfb-run -a` 1327 of 1332, then the five
(two ABI counts, three GTK tests) fixed and 15 of 15; rustdoc `-D warnings --document-private-items` 0; fuzz
`cargo check --bins` 0; conformance after `git add -N` 0, 474 passed. Tier 2 is the train's.

**Left.** The WinUI print dialogue (`doc/todo/30` N4) owes the box and `quorra_print_size`; `quorra` has no printer
(ADR 1180 section 6). §12.2 stays `departed`.
