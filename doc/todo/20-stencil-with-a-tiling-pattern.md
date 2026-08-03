# A stencil painted with a *tiling* pattern

Status: refused by name at runtime.
Priority: 20
Corpus: 2 documents, 5 images
Clauses: §8.9.6.2 with §8.7.3, §11.5.2
Code: `crates/pdf-model/src/image.rs`, `crates/pdf-model/src/content.rs`

An image mask (§8.9.6.2's stencil) is drawn as an image whose samples carry the fill colour, and
**no image sample can carry a pattern**. So `scn` with a pattern name used to leave `state.fill`
at its initial transparent black and the page drew blank while reporting nothing — which is what
`issue13372.pdf` did until the hundred-and-eighty-first session.

The **shading** half is done (ADR 0151): the stencil becomes a §11.5.2 alpha soft mask and the
pattern fills the image's unit square through it. A **tiling** pattern is a replayed content
stream rather than a paint, so it needs the tiling machinery to accept a mask — `tile` builds a
clip from the filled path and would have to take a soft mask beside it.

Refused by name now, at a cost of two documents on the corpus's incomplete list. It used to be
painted in whatever colour the state last held.
