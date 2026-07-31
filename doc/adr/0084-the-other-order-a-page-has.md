# ADR 0084 — The other order a page has

Status: accepted, 2026-07-31.

## Context

Every reading of a page this project had done was the content stream's. §14.8.2.5 says a tagged
page has a second one:

> Page content order shall be defined by the sequencing of graphics objects within a page's
> content stream.

> Logical content order -the ordering for semantic purposes -shall be defined by a depth-first
> traversal of the document's logical structure hierarch y.

The structure tree has been read since the seventy-eighth session and `Tree::walk` has been a
depth-first traversal since. What was missing was the *join*: which bytes of the readback belong
to which marked-content sequence, without which the second order cannot be applied to anything.

## Decision

`Interpretation::marked` records one span per `BDC` … `EMC` whose property list states an
`/MCID` — the same mechanism `artifacts` already used, closed at the same point and after the
same `/ActualText` replacements. `Tree::logical_order` filters the walk to one page's content
items, and `Tree::logical_text` assembles the readback through them.

Two of the clause's own restrictions are applied rather than stored:

- **Only structure elements are in the logical order.** A sequence the tree does not reach is
  left out of `logical_text` entirely, and NOTE 3 states the case that would otherwise tempt a
  reader — "[a]rtifacts not contained within an Artifact structure element are not considered
  part of the logical content order".
- **Annotations are in the order and not in the string.** §14.8.2.5.2 says an annotation "is not
  interleaved within the page's content stream" and that its position "is determined from the
  document's logical structure", which is §14.7.5.3's object reference. `logical_order` returns
  those in place; `logical_text` skips them, because an annotation's text is its `/Contents` or
  its field's value rather than a range of the page's readback, and splicing it in would produce
  a string that is neither.

Nothing about *drawing* changes. The display list is the stream's order and always was.

## What the corpus says about a "should"

§14.8.2.5.1 says the two orders "should coincide". That is measurable, and the measurement is the
session's finding:

- 89 documents have a structure tree; 77 mark content on page one.
- **72 of the 77 coincide. Five do not**, and every one is a real reordering rather than a
  reader's defect — the byte counts match exactly and the sequences are permuted.
  `prefilled_f1040.pdf` reads its title first and shows a margin note first; `pdfjs_wikipedia.pdf`
  puts its heading in a different place; `annotation-button-widget.pdf` swaps "checked" and
  "read-only".
- 3 marked-content sequences across the whole corpus are unreachable from any structure element,
  which is text in neither order.

## And a "shall" the corpus mostly ignores

§14.8.2.6.2 puts the word breaks on the *document*: "any white-space characters that would be
present to separate words in a pure text representation shall be present in the tagged PDF
representation of the text", with NOTE 1 drawing the consequence that a processor "can determine
word breaks without having to rely on heuristics based on information such as glyph positioning
on the page".

This tree has relied on exactly that heuristic since it first read text back. Rather than argue
about it, it now counts: `Interpretation::inferred_separators` is how many separators came from
position rather than from a character. Over the 77 tagged first pages it is **2392, and only 20
of the 77 need none**.

So the heuristic is not a departure from §14.8.2.6.2 — it is a defence against documents that
claim to follow it and do not, and removing it would empty most of the word breaks out of the
readback on 57 of 77 pages that call themselves tagged. That is the difference between a
documented departure and a documented *robustness measure*, and the number is what tells them
apart.

## Consequences

- `silent` falls 71 → **66**: §14.8.2.5, §14.8.2.5.1, §14.8.2.5.2 and §14.8.2.6.2 close, and
  §14.8.2.6 becomes `partial` for the inference above.
- Clause 14 has **two** silences left: §14.8.5.6's `PrintField` and §14.7.7's worked example.
- `Tree::logical_text` is the first consumer of the structure tree that produces something a
  person could read, which is what four sessions of reading §14.7 were for.
- No gate moves: neither order changes a mark on a page.
