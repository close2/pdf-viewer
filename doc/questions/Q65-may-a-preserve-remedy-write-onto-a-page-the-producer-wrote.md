# Q65 — May a `preserve` remedy write onto a page the producer wrote?

Source: session 1085, building `doc/adr/1099`.

## Why it needs the owner

`CLAUDE.md`'s authoring exclusion was amended a third time on 2026-09-12, by `A58` and
`doc/adr/1014`: an archival conversion may **append a page** carrying content the document already
holds where the alternative is losing that content outright. The same paragraph keeps the far side
shut in one sentence: *the watermark stays on the far side: it composes new content over pages, and
nothing here reaches it.*

ISO 19005 forbids an annotation whose subtype it does not admit, and the only rewrite that meets
the clause removes it. Its normal appearance is a form `XObject` **the producer wrote**, and
§12.5.5 fixes, to the matrix, where a reader draws it. Session 1085 keeps those marks by appending
a page that states the source page's boxes and invokes the same stream under the same matrix. That
is inside the permission as it stands, and it works: the corpus witness comes out conforming, with
the marks at the producer's coordinates on a page of their own.

**The smaller answer is one this round could not take.** Append `q AA cm /X Do Q` as a further
entry of the *annotation's own page* `/Contents` array, and the page count does not change, the
rendered page is identical rather than equivalent, and nothing is appended at all. Every mark is
the producer's; the operators added are the placement §12.5.5 already computes, so by
`doc/adr/0816`'s own test — *does the operation invent marks?* — the answer is no, and the
watermark's own sentence says "composes **new** content over pages", which this does not.

Against that: the *operation* is writing into a content stream a producer wrote, and that is what
the sentence was put there to stop, whoever the marks belong to. The amendment names appending a
page and nothing else.

## What the tree does meanwhile

`doc/adr/1099`: the appended page. It conforms, it keeps the marks, and it costs a page.

## Two things that would have to be settled with a yes

Neither is a reason to say no; both are the honest cost of the question.

- **§8.4.4 requires `q` and `Q` to balance within a content stream**, and files exist that do not.
  Operators appended after the producer's would inherit whatever graphics state the producer's left
  — a leaked `cm`, an unclosed clip. The construction that survives it is a `q` prepended as its
  own stream and a `Q` before the placement, which restores the initial state unless the producer's
  content pops further than it pushed; a document that does would have to be refused by name.
- **Drawing order.** §12.5.5 composites an annotation's appearance over the page content *and any
  previously painted annotations*. Marks moved into the page content go under every annotation that
  remains, so an annotation that drew over another would change what a reader sees. Rare, and
  detectable: the overlap is computable from the rectangles.

## Recommendation

**Rule on the sentence rather than on this site.** If "composes new content over pages" is read as
this round read it — the *provenance* of the marks is what the line is about — then writing a
producer's own appearance back onto the producer's own page is on the near side, and the two costs
above become refusals with sentences. If the line is about the *operation* instead, then the
appended page is the whole of what this program may do here and ADR 1099 stands as it is.

Either answer is workable and the difference is one page per removed annotation. What is not
workable is leaving it unruled, because the same question is the next one for every other
`preserve` site whose content came off a page.
