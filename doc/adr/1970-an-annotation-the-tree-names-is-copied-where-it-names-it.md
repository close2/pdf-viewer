# 1970 — An annotation the tree names is copied where it names it, and a press on what nothing reaches says why

Session 1567, slot 4 of batch ninety, a clause round. Status: **accepted, built**. Context: ISO
32000-2 §14.7.5.3, §14.8.2.5.1, §14.8.2.5.2, §7.7.3.3 Table 31, §12.7.3, Table 224; ADRs 1100,
1948, 1957.

## 1. The premise did not hold for either document it named

ADR 1948 §4 said `160F-2019.pdf` and `bug1868759.pdf` copy in page order "for that reason alone":
an annotation the tree claims by §14.7.5.3 reaching the readback without its object. Read from the
files, neither is that case. `160F-2019.pdf` states no `/MarkInfo`, no `OBJR` anywhere in its tree,
and no `/StructParent` on any widget; the bytes no route accounts for are the reset button's caption
"Réinitialiser", an untagged widget. `bug1868759.pdf` (no `/MarkInfo` either) tags only empty
`/Link` sequences and 43 link `OBJR`s; every one of its 1949 readback characters lies outside every
`/MCID` sequence. Both are ADR 1948 §2's text nothing claims, which keeps a copy in page order by
decision, and both still do.

## 2. The case the clause states is built anyway

§14.8.2.5.2: "The position of an annotation in the logical content order is determined from the
document's logical structure." An annotation's appearance reads back into `Interpretation::text`
after the page's content and carries no `/MCID`, so before this the only record of its text was a
bare range kept for §14.8.2.2.2's artifacts by absence, and `Tree::logical_range` could not account
for those bytes: a tagged form copied in page order whenever a field held a value.

`Interpretation::objects` now carries each claimed annotation's object with its range
(`content::ObjectSpan`), and `logical_range` takes the three span kinds as one `ContentSpans`. An
`OBJR` naming one of them gives its text a piece where the traversal reaches it. Two rules, each
from the clause: an object named twice gives its text once (Table 358's NOTE 2, "just a single
object reference suffices"), and bytes a marked-content sequence inside the appearance covers stay
that sequence's (§14.7.5.3's NOTE 1, "it is instead handled as a marked -content sequence").

**Recorded on a document making §14.8.1's claim only.** The claim is asked of the tree the
interpreter keeps for the artifacts by absence, which exists only under `/Marked true`. A form
without it copies in page order, the complete answer. Over the corpus, two first pages whose only
content items are their fields (`annotation-text-widget.pdf`, `annotation-choice-widget.pdf`) now
copy in the tree's order; withheld, they do not. The 58 of the 78 pages that mark content are
unchanged, which is section 1 measured.

## 3. A press on what nothing reaches says why, at every level

`report_unreachable` says Table 31's and §12.7.3's (or Table 224's) sentences at a page's open, and
only where a runner was supplied; a press on either kind does nothing whatever the level — a
dictionary in `/Annots` has no identity to raise an event on, and a widget no form reaches is no
field. `ViewState::report_unreachable_press` says the open's own sentence for the topmost
interacting annotation under a press, and `viewer_core`'s press calls it. The view state keeps each
sentence once, so a reader at `off` hears it at the first press and one at `on` has heard it at the
open.
