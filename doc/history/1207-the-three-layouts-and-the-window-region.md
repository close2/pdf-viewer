# Session 1207 — the three layouts, and the window region

The batch's host-UI round. Contract: §12.3.5 and §12.3.5.1, the last collection residues.

## What the clause turned out to say

§12.3.6's three remaining named layouts are addressed to a processor entirely in `should`s, and
Table 160's selection rule is conditional on capability — so reporting them was legal and building
them was owed. What each *is* was the surprise. The brief guessed `Linear` might be this panel's
list in `/Sort` order; the entry says "a large size preview of **one** file attachment in the
collection" with its metadata beside it. And the three were undrawable for a reason nobody had
named: not the arrangement but the **picture**. Each is built out of pictures of the attachments,
and this program had none.

§12.3.4 is where the standard says what a picture of a document's page is, so
`Query::AttachmentPreview` opens the named attachment and answers its first page's `/Thumb` — one
file at a time, one level deep, under the stream bound that already bounds a bomb. A file that
states none keeps its Table 44 kind and the panel says so: trap 5's rule, and the mirror of the
mistake ADR 1215 corrected.

## Two stale claims, found by reading the tree

Both ledger rows said `/Colors` and `/Split` were "named by `unsupported_presentation` rather than
dropped in silence". **Neither entry was named by that function or by anything else** — a four-session
claim about a report nothing made. `panel::unused_furniture` says them now.

ADR 1168 wrote that `/Split` "describe[s] a widget none of the three windows has". **`viewer-gtk` has
held a `gtk4::Paned` between the panels and the page since before that round, and `viewer-qt` a
`QSplitter`.** The departure survives on a different reason — §12.3.5.1's second area is "a preview of
the initial or currently selected document" and this window's is the container's own pages — and
Table 158's `N` stops being a departure at all: it says the window is *not* split, which a window with
no bar can obey. All three now do.

## Rows

§12.3.5 and §12.3.5.1 `partial` → `departed` (ADRs 1251, 1252), and §12.3 with them, forced by the
gate once its last two children settled. Left for a later round: whether §12.3.6's `Linear` should let a person reach a second attachment
through something other than the list under it. The clause states nothing; the list is this
round's choice.
