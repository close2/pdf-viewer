# 0748 — A list that carries an earlier one on, and the ledger's only `silent` row

Status: accepted.
Context: `doc/conformance/ledger.toml`'s §14.8.5.5, put on `silent` by ADR 0743 and the only row
holding that status.

## What the status meant, and why it was the round's subject

`doc/ledger-and-claims.md` defines `silent` as *not implemented, and nothing says so*. It is the
one status this project treats as a defect rather than as a position: principle 3's "unsupported
input must stay loud" and `doc/traps/parsers-and-streams.md`'s trap 5 are the same sentence, and
`doc/todo/README.md` closes with "[w]hat is not implemented has a file … every one of them is
*reported* at runtime rather than silently skipped". A row on `silent` is that promise broken.

The population had been zero for four hundred rounds, which is the reason the row is worth an ADR
of its own: the zero was not evidence that nothing was owed. It went to zero because every row
holding the status was paid off and stayed there because nothing re-read the rows that say nothing
is owed — §14.8.5.5 sat on `inapplicable` under a note that named one of Table 382's three entries
and argued the whole clause from it. ADR 0743 found the other two by reading an erratum.

## What the clause states, and who it is addressed to

ISO 32000-2 §14.8.5.5 separates its two paragraphs by *audience*, which is what decides this:

> If present, the List attributes described in "Table 382 -Standard list attributes" shall appear
> in an L (List) element. These attributes control the interpretation of the Lbl (Label) elements

— that is `/ListNumbering`, and the argument for calling it inapplicable stands: it says how the
labels *were* numbered, and the labels themselves are `Lbl` elements holding marks the page already
shows. Then:

> The ContinuedList and the ContinuedFrom attributes described in "Table 382 -Standard list
> attributes" control the interpretation of the L element as it relates to other L elements that
> are not its immediate parent.

*The interpretation of the L element* is this program's job the moment `StandardType::List` reaches
a screen reader as a list. Table 382 states the two entries:

| entry | |
|---|---|
| `/ContinuedList` | "A flag specifying whether the list is a continuation of a previous list in the structure tree ( true ), or not ( false ). Default value: false . If the ContinuedFrom attribute is not present, the continuation is from the preceding list at the same level in the structure hierarchy." |
| `/ContinuedFrom` | "The ID … of the list for which this list is a continuation." |

Errata Collection 3's Issue #346 carets *not inheritable; * into both cells and into neither of
`/ListNumbering`'s, so the reader asks `Tree::attribute` rather than `Tree::inherited_attribute`.
A list nested inside a continuing list has not been said to continue anything.

## What was built

**`pdf_model::structure::Tree::list_continuation`** answers `Option<ListContinuation>` — `From(id)`
where the element names its predecessor, `Preceding` where the clause names it instead.

**`pdf_model::structure::list_predecessors`** resolves that over a walk: it takes the walk's `L`
elements in order, with each one's depth, its Table 355 `/ID` and its continuation, and answers
which position each continuing list continues. Two rules bound it, and both are the clause's own.
The population is the walk's **lists**, because `/ContinuedFrom` names "the ID … of the **list**".
And a predecessor is always **earlier in the walk** on both routes: the fallback says *preceding*,
and a `/ContinuedFrom` naming a later list contradicts what a continuation is. The second rule is
also what lets a caller that is building a parent-first list point at the answer without a search,
which is the constraint Table 384's `/Headers` are already resolved under.

**"The same level in the structure hierarchy" is read as the same depth**, which is what the clause
says. In the shape the entry exists for — two lists with other content between them — a
same-parent reading would give the same answer, because such lists are siblings.

**`viewer_core::AccessibilityNode` gains two fields**, `continues_a_list` and `continued_from`, and
they are two rather than one deliberately. The predecessor is frequently not in the answer at all:
this answer is one page's, and a list split across a page boundary has its predecessor pruned away
exactly as a table's header row is. WTPDF's own NOTE on the pair says the flag is worth having
anyway — "[t]here is value to the ContinuedList attribute even when the previous list is not
present, since it would help to explain the partial nature of the content, for example, partial
numbering." A single `Option<usize>` would have made those two cases one.

**`viewer-accessibility` publishes both halves.** The description says *a continuation of an
earlier list*, with *on this page* added where the predecessor is in the answer — the half a person
hears. And AccessKit's `FlowTo` runs from the predecessor to the list that carries it on, which is
the half a client follows; the relation is published on the element the reading *leaves*, so the
answer's direction is inverted once, in `tree::continued_by`, rather than searched for per element.

A host cannot work any of this out. The attributes live in §14.7.6's attribute objects and class
map, which only the confined side reads, and nothing on the page says a list carries on — the
numbering restarting at 1 is precisely what a producer writes `/ContinuedList` to contradict.

## The choice this round had to make, and it is a choice

Table 382 defaults `/ContinuedList` to `false`, and `/ContinuedFrom` is "the ID … of the list for
which this list is a continuation" — a sentence that asserts the element *is* one. The standard
does not say which a file stating only `/ContinuedFrom` means. The reader takes it as a
continuation, because discarding the only statement the file made is the silence this work exists
to end; and an explicit `/ContinuedList false` beats it, because that is the producer saying `no` in
the entry whose job is to say it. The census counts that population apart so the choice stays
measurable.

## What the corpus says, and it says nothing

`pdf-model --example list_continuation_census` walks a structure tree and counts both entries, the
resolutions each route produces, and — the probe trap 11's rule asks for — elements stating either
entry that are **not** `L` elements. Over the pdf.js corpus, `doc/` and the four corpora: 1245
documents opened, 153 with a structure tree, 1566 `L` elements, **not one** stating either entry.
Over the SafeDocs crawl: 65 967 opened, 23 501 with a structure tree, 196 297 `L` elements, **not
one**.

That is the same position §14.8.5.6's `PrintField` is in, and it is the reason this belongs to the
coverage question rather than the robustness one. A corpus cannot rank a requirement no document
exercises; the entries are PDF 2.0's, the tagging profiles that ask for them are newer than most of
what has been crawled, and a reader that waited for a witness would be waiting on producers rather
than on the standard. `CLAUDE.md`'s two denominators are exactly this distinction, and the zero is
recorded rather than glossed: it is what the instrument said, and the instrument is committed so
the next round can ask again.

## What was not done

The count is **printed** by `accessibility_census` and not ratcheted. `doc/todo/05`'s rule is that
a number enters a gate once it has held across rounds, and this one is a round old.
