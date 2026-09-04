# 0863 — A piece begins where the outline lands, and the front matter is a piece too

Session 910. Status: **accepted**. The eighteenth decision record of RFC 0002's implementation, on
the long-lived branch `round-867`, and the last mode of the suite's first verb. ADR 0862 is the
carrying this one rests on.

## Context

RFC 0002 §6.1 gives `split` four ways of saying where the cuts are, and three of them landed in
session 886 (ADR 0818): one file per page, pieces of *n*, one file per comma-separated group. The
fourth — `--at-bookmarks[=depth]`, "the one on this list no surveyed CLI does first-class" — did
not, and ADR 0818's last line said why: "it wants `pdf_model::retrieval::sections`, which exists,
and an outline subset, which does not."

The outline subset exists now, so what is left is the rule for where a piece begins, and the one
question the RFC left open about it.

## Decision

### 1. The rule, in one sentence and its three edges

**A piece begins at every selected page that an outline item at the stated depth or shallower
resolves to, and runs to the page before the next such page.**

The resolution is `pdf_model::retrieval::sections` — §12.3.3's hierarchy through §12.3.2's
destinations, ADR 0257's machinery, reused rather than rebuilt as the RFC asks — so a named
destination goes through §12.3.2.4's two tables here exactly as it does when a reader follows a
link. `depth` counts §12.3.3's levels from 1, because that is how a person counts them and how the
flag is spelled; a `Section`'s own `depth` counts the same levels from 0.

Three edges, each decided rather than fallen into:

- **The pages before the first mark are a piece, with no title.** A `split` whose pieces do not
  cover the selection has lost pages, and front matter ahead of the first bookmark is exactly that
  case — a title page and a table of contents are what would have gone missing. This is the RFC's
  proposal amended by the verb's own meaning rather than by preference.
- **Two items landing on one page start one piece, not two**, and the *first* in the outline's own
  order names it. An empty piece is not a file, and taking the first makes the answer a function of
  the file rather than of the iteration order.
- **A document whose outline resolves nowhere at that depth is refused by name**, as
  `Refusal::NoBookmarks`, at exit 2. The same shape as `Refusal::Selection` and for the same
  reason: the request is well formed and the document is readable, but it does not hold what the
  plan asked for. §12.3.3 makes an outline optional, so most documents land here — and a verb that
  answered by writing one piece would have cut nowhere while saying it cut at the bookmarks
  (trap 5).

### 2. The RFC's open question, answered by ADR 0862 rather than by this record

RFC §6.1 asks "[w]hether a piece keeps the source's whole outline (grayed context) or only its own
subtree — proposed: own subtree". The subset ADR 0862 derives is not quite either: it is the items
that **reach the piece's pages**, which for a chapter file is its own subtree and its ancestors, and
for the front-matter piece is whatever landed there. The proposal's answer and the derived one
coincide wherever the outline is a table of contents, and where they do not, the derived one is the
one Table 151's `/Parent` forces.

### 3. `%t` is a title now, and the pattern says which piece has none

`Pattern::names_a_title` was refused up front by `split` with the sentence "a piece of a document
has none until --at-bookmarks lands". It is accepted for this mode and still refused for the other
three, because a piece cut every ten pages has no title to name. The leading untitled piece falls
through to `Pattern::expand`'s documented answer — "[a]n escape the fill cannot answer … expands to
its ordinal" — so `'%d-%t.pdf'` writes `01-1.pdf` for the front matter and `02-Foreword.pdf` after
it.

The flag itself needed one thing the argument reader did not have: an **optional** value written
inline. `--at-bookmarks` cannot take the next argument, because `--at-bookmarks in.pdf` would then
be a depth; so `OPTIONAL` is a third list beside `VALUED`, holding one flag.

## Consequences

- **The corpus walk asks the mode its own two questions**, derived rather than compared: the pieces
  cover the document's pages exactly once — a verb that lost the front matter would still look
  right piece by piece — and every piece but the leading one begins on a page a level-1 item
  resolves to. Over the 974, **23** documents have an outline naming two or more pages at level 1,
  and all 23 pass both with nothing refused.
- **The foreign readback gained a sixth lane.** `--at-bookmarks` is the same verb writing a
  *different shape* — a piece of several pages carrying an outline, page labels and named
  destinations, where `split`'s lane writes one page — so poppler, mupdf and `qpdf --check` are
  asked about it separately. It fits the existing comparison for free: the mode's **first** piece
  always states the source's page 1 as its own page 1, whether it is the front matter or the piece
  the first mark begins.
- `split` is complete against RFC §6.1. What the verb still does not carry is ADR 0862's last
  paragraph.
