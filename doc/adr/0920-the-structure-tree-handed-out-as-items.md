# 0920 — §14.7's tree handed out as items, and the artifact filter that could not be applied to it

Session 940. Status: **accepted**. `quorra-retrieve` gained a `structure` question and lost a
silent corruption: `--logical --no-artifacts` had been cutting characters out of the text it
returned, in the width of the running head it thought it was removing.

## Context

The owner bought ISO 19005 parts 2 and 4 (`doc/questions/A16`) and asked for them to be prepared
so that the project can use them, adding: "maybe our own viewer is suitable for extracting text".
It is — both files are AES encrypted, both are tagged, and `pdf_model::structure` already reads
§14.7.2's tree. Extracting them with `quorra-retrieve` rather than with `pdftotext` is what found
both halves of this decision.

## The corruption

`Retrieval::read_page` did two things in sequence: it took §14.8.2.5's logical order where the
caller asked for it, and it then subtracted §14.8.2.2's artifacts from the string. The second
step's ranges — `Interpretation::artifacts` — are offsets into the **raw readback**, and the
logical reading is a *different string with the same characters in another arrangement*. Applying
one to the other removes a run of characters as wide as the running head, from wherever that
head's offsets happen to land in the rearranged text.

The crate already knew this. `Retrieval::section`'s doc comment says it in as many words —
"[d]ropping §14.8.2.2's artifacts or taking §14.8.2.5's order changes which characters are where,
so an offset computed in one of those is meaningless in the other" — and computes its section
edges twice for exactly that reason. The page path did not, and the two flags had presumably never
been passed together.

What it cost, on the two documents this session read: the first 62 characters of ISO 19005-2's
page 26 — "All conforming readers shall use the embedded fonts, rather tha" — and the same width
off the top of every page of ISO 32000-2. Silently: the answer came back shorter with no flag
saying so, which is the failure `CLAUDE.md` principle 1 names.

## Decision, and the clause that makes it a one-line fix

**The artifact filter applies to content order only.** Not because the combination is unsupported,
but because §14.8.2.5.1 NOTE 3 has already done the work:

> Artifacts not contained within an Artifact structure element are not considered part of the
> logical content order.

A running head, a folio and a licence stamp are marked content the structure tree does not reach,
so a logical reading never had them. The flag has nothing left to do.

**The one case where it is not a no-op is deliberate.** An artifact that *is* inside an `/Artifact`
structure element is part of the logical content order by the same NOTE, and it stays. A caller
asking for both gets the clause's answer rather than the flag's, and that is the better of the two
answers to give: the order was asked for by name.

## The `structure` question

Preparing a specification wants more than a page's string: ISO's paragraphs, its clause headings,
its lists and its tables are all *stated* by the file, and reading them off glyph positions when
the document says them outright is guesswork over evidence. `Retrieval::structure` walks the tree
once and hands out what it finds.

**It hands out items, not elements with text.** The shape that suggests itself — one entry per
element, carrying the text its content items marked — loses the interleaving, and the interleaving
is load-bearing. §14.7.5.1 makes an element's children "zero or more items of the following kinds" —
"[o]ther structure elements" and "[r]eferences to content items" — and ISO 32000-2
sets a URL as a `Link` inside a sentence: an answer that gathered each element's text into one
string reads that URL *after* the full stop. So the answer is the walk itself — `Element` opening a
depth, `Text` carrying one sequence's readback — and a consumer reassembles a paragraph by
concatenating in order.

Two invariants are asserted rather than described, in `tools/pdf-retrieve/tests/retrieval.rs`:
the items of one page, joined, are that page's `Tree::logical_text`; and the standard's own tree
contains elements whose text is broken around a child.

## Consequences

- `tools/pdfa-text.py` renders both bought parts to Markdown from this answer, and the coverage
  check it prints — every word the pages draw that did not reach the Markdown — comes to **zero**
  for part 4 and to the fifty em dashes part 2 uses as list markers.
- The `structure` question is not PDF/A's. It is what any consumer wanting a document's own
  outline of itself asks for, and it is a reader over data this tree already had.
