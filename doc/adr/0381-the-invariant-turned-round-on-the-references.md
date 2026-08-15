# 0381 — The invariant turned round on the references, and the family member outside it

**Status.** Accepted.

## Context

The oracle's **contradicted** bucket is the list of pages where the reference consensus disagrees
with this tree. Every page on it sits in a group in `crates/pdf-model/tests/oracle.rs`, and each
group's doc comment is its diagnosis. Trap 1's standing warning is that **a group's name states a
hypothesis rather than a diagnosis, and it has been wrong ten times out of ten** — so the work on
this bucket is not "find the undiagnosed pages", it is to re-open a page and check that what its
group says is what the page is.

Ranked by `doc/habits.md`'s ranking — our worst measurement over the bound it is held to — the head
of the list is `CONTRADICTED_SHARED_JBIG2_DECODER`, seven pages, whose argument is trap 9's third
shape: `mupdf` and `ghostscript` both link `jbig2dec`, so on a JBIG2 page they are one
implementation and their agreement is not the evidence the oracle takes it for.

That argument is correct and it is also *negative*. It says why their agreement proves nothing; it
does not say what is true. `doc/todo/00`'s "what a group must say" asks for the second thing — the
owner's sentence is *even if the oracle cannot agree, we should be able to determine what is
actually true, based on the spec* — and the group answered it by pointing at `tests/jbig2.rs`,
which checks *us* against the corpus's own invariant: the pdf.js corpus holds a family of
documents that are one drawing encoded through nearly every coding path ISO/IEC 14492 defines, and
every one of them must decode to the same pixels.

Two things were left undone by that, and both were found by opening the pages.

## The first: the invariant is an instrument, and it had only ever been pointed at us

The corpus's invariant is a property of the *documents*, not of this renderer. Any program that
claims to decode JBIG2 can be asked the same question, and the answer involves no comparison
between renderers at all — **each is compared only with itself**, which is why principle 5 is not
in tension: no reference is being treated as a source of truth, and none is being matched.

Asked with `doc/todo/02` §2's own reference command lines over the family, grouped by the hash of
`magick <png> -depth 8 -colorspace Gray txt:-`:

| | distinct images over the family | self-consistent on |
|---|---|---|
| ours | **1** | all of them |
| `poppler` | 8 | 79 |
| `mupdf` | 6 | 71 |
| `ghostscript` | 6 | 71 |

And the image `mupdf` and `ghostscript` produce on the 71 they are consistent about is
**byte-identical to ours**: `magick compare -metric AE` between our render of
`bitmap-halftone-composite.pdf`, which `jbig2dec` gets wrong, and `mupdf`'s render of
`bitmap-halftone.pdf`, which it gets right, is **0** — and 0 again against `ghostscript`'s render
of `bitmap-template1.pdf`.

So the disagreement on these seven pages is not two readings of ISO/IEC 14492. It is one decoder
that answers differently depending on how the image was coded, on a quarter of the family, against
a decoder that does not — and the answer the first one gives when it is self-consistent is ours.
That is the positive statement the group was missing, and it is available from the documents alone.

`poppler` cannot be compared byte-wise with anybody: it smooths the image on the way to the page,
198 grey levels against our two at one device pixel per sample. Its self-consistency is still
measurable, and it fails the invariant on 17 of the family.

**The per-page picture, which the group had summarised wrongly.** Its note said "on four of them it
decodes nothing and renders a blank page, on two it produces the drawing strewn with noise blocks".
Measured page by page (ink of 255, `-alpha off`):

```text
                                             ours  poppler   mupdf      gs   hayro
  bitmap-halftone-composite                17.495   19.253  22.594  22.594  17.495
  bitmap-refine-page-subrect               17.495   17.589  21.052  21.052  17.495
  bitmap-symbol-context-reuse              17.495    0.000 255.000   0.000  17.495
  bitmap-symbol-symhuffrefineone           17.495   17.589  19.422  19.422  17.495
  bitmap-symbol-texthuffrefinecustom       17.495   17.589   0.000   0.000  17.495
  bitmap-symbol-texthuffrefinecustomposdims 17.495  17.589   0.000   0.000  17.495
  issue20439                               17.495   17.589   0.000   0.000  17.495
```

Three blank, three with extra ink, and one — `bitmap-symbol-context-reuse.pdf` — where `mupdf`
renders the page *entirely black* while `ghostscript` renders it white, the only one of the seven
where the two are not byte-identical to each other. **And six of the seven are silent**: asked
again, only that page produces a warning from either program. A note that generalised its `NYI`
log to all seven was describing one page.

## The second: a member of the family whose name does not say so

`tests/jbig2.rs` built its population from the `bitmap-` filename prefix. `issue20439.pdf` is
1 300 bytes, one `/JBIG2Decode` image XObject on a `[0 0 399 400]` page, and it is the same
drawing — our render of it and our render of `bitmap-halftone-composite.pdf` differ in **zero**
pixels, as do `mupdf`'s renders of it and of `bitmap-symbol-texthuffrefinecustom.pdf`.

So the one instrument in this tree that can judge a JBIG2 decode was not run on a document the
oracle lists as *contradicted*. That is the shape worth naming: **a population defined by a naming
convention silently excludes the members that were named after something else**, and the excluded
member is likelier than average to be the interesting one, because a file named after an issue is
a file somebody filed an issue about.

## Decision

1. **`tests/jbig2.rs` admits family members by name as well as by prefix.**
   `FAMILY_MEMBERS_NAMED_OTHERWISE` holds `issue20439.pdf` and the ratchet rises with it. The
   admission is self-checking and needs no argument to be trusted: the test groups the renders by
   value and demands exactly one group, so a document that is some *other* picture makes it report
   two and fail.

2. **The reference half of the invariant is recorded, not gated.** It costs about 300 reference
   renders and it is a measurement about somebody else's decoder, so it is written into the group's
   note and this ADR with the recipe beside it rather than made a gate. What is gated is the half
   that is about us, which is where a gate belongs.

3. **The group stays listed and its members stay contradicted.** Nothing about this changes what
   the oracle should print: if `jbig2dec` is fixed these pages leave the list, and if our decode
   changes they change too. What changed is that the group now says what is true rather than only
   why the references' agreement is not evidence.

## Consequences

- One number in `tests/jbig2.rs` moved and no pixel did. The gate reports the family entire.
- Three other groups were re-opened in the same round and each gained the evidence it was missing
  rather than a correction: `CONTRADICTED_LINK_BORDER` (`hayro` draws no border either, and the
  colour both drawing renderers paint is the annotation's `/C` rather than a default),
  `CONTRADICTED_REFERENCES_DREW_NOTHING` (`ghostscript`'s log says what the note claims **only
  without `-q`**, which the oracle passes — so a round reading the gate's own invocation would have
  found silence and called the note wrong), and `CONTRADICTED_DEVICE_CMYK_CONVERSION` (its ramp
  re-sampled at `c` = 0.5: ours 127, `poppler` 128, `mupdf` 109, `ghostscript` 108, `hayro` 109).
- **A caution earned twice in one round**: a reference's silence is a fact about the invocation
  before it is a fact about the renderer. `-q` hides `ghostscript`'s diagnosis, and six of seven
  `jbig2dec` failures are silent under any flag. Trap 3 says to check what question a reference is
  being asked before reading its answer as a verdict; this is the same rule for the answer it
  *does not* give.
