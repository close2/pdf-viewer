# ADR 0224 — Two tables spell `/RC`, and one of them marks the page

Status: accepted, session 387.

## What was decided

**Table 177's `/RC` generates a free text annotation's appearance where the file states no
`/Contents`.** `popup::rich_text` became `pub(crate)` and `appearance::free_text` calls it; the
characters are taken and the XFA markup is not, which is ADR 0199's reading applied unchanged to
the *second* of the standard's two `/RC` entries.

Until this session a free text annotation stating only `/RC` drew nothing and reported nothing.
On this subtype that is a blank page: §12.5.6.6 says the annotation "displays text directly on
the page", so the text *is* the annotation.

## Why the two entries are not one decision made twice

They carry different `shall`s, and the standard says so beside them. Table 172's, which ADR 0199
implemented:

> A rich text string (see Adobe XML Architecture, XML Forms Architecture (XFA) Specification,
> version 3.3 ) that shall be displayed in the popup window when the annotation is opened.

Table 177's, which this session implemented:

> A rich text string (see Adobe XML Architecture, XML Forms Architecture (XFA) Specification,
> version 3.3 ) that shall be used to generate the appearance of the annotation.

and its NOTE separates them outright:

> As freetext annotations do not have an open state this cannot apply to the popup window as
> described for the RC key in "Table 172 - Additional entries in an annotation dictionary specific
> to markup annotations".

So the *extraction* is one function — a rich text string is a rich text string — and the
*destination* is a clause's decision each time. That is why `rich_text` was widened rather than
copied, and why it now documents both callers.

`/Contents` still outranks `/RC`, which is §12.5.6.2 NOTE 1's own reading: the two are "expected"
to be textually equivalent where both are present, and a plain text string is what this crate can
hand over without reading a specification it does not have. **No document stating `/Contents`
changes at all.**

## What principle 5 excludes here, and what it does not

`CLAUDE.md` excludes XFA, and Annex K is where the standard hands that over. What it excludes is
the *format*: a `<span>`'s style, a colour, a face, a size. What it cannot exclude is a `shall`
addressed to this processor about what appears on a page. The line ADR 0199 drew — characters in,
formatting out — is the same line, and drawing it a second time is what stopped this entry being
refused on a reason that was never about it.

Table 177's `/DS` keeps the exclusion, and the difference is worth stating rather than assuming:
it is "[a] default style string, as described in … XFA", and no sentence in the clause makes it
displayed. A permission declined and a requirement met are different answers.

## The corpus cannot exercise it, and that was counted

`examples/markup_text_census` gained a subtype breakdown for exactly this question. Over the
corpus's 974 files, the 964 that open, every page rather than page one:

- 71 annotations state `/RC`, 18 with no `/Contents` — the popup population ADR 0199 measured.
- **22 are free text annotations stating `/RC`, and every one of them also states `/Contents`.**

So not one corpus page changes, and the oracle, the corpus gate and the quorra gate cannot see
this work at all. That is trap 8's converse and it is the same position ADR 0199 was in: the
module's own tests are the whole defence, and
`a_free_text_annotations_rich_text_draws_where_it_states_no_contents` was run against the code
without the change and fails there with "nothing was drawn at all".

## How it was found, which is the part worth keeping

Not from the corpus, and not from reading §12.5.6.6. `doc/todo/01`'s **second sweep** — an entry a
row claims is unread, grepped against the tree — printed §12.5.6.6's note saying `/RC` and `/DS`
"are XFA rich text, which principle 5 excludes" beside a `LIVE /RC: crates/pdf-model/src/popup.rs`.

**It is the fourth row to carry that sentence about an `/RC`.** §12.5.6.2's was corrected in
session 342, §12.5.2's in session 375, and neither round read this one — which is the sweeps' own
oldest lesson (a mechanism gets one row per clause that mentions it, and correcting one leaves the
others lying), arriving a third time on the same three characters.
