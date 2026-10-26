# Q633 — When is XFA's "later", and what is its first step?

Asked by round 1768, which re-read the ledger's `out-of-scope` rows for scope
(`doc/reviews/1768-the-inapplicable-and-out-of-scope-rows-re-read-for-scope.md`, finding 6).

## The rows, and the reason they record

**Annex K, K.1 and K.2**, `out-of-scope` on `CLAUDE.md`'s XFA exclusion, which rests on §K.1's own
permission to decline and, since `A254`, on the authoring exclusion as well. §K.2's consistency rules
make a hybrid file's `AcroForm` the form, and every window says once per document that it carries an
XFA form and what is shown instead (ADR 1932). Of the 970 documents in `doc/pdf.js` and
`doc/corpora` that open, 7 state an XFA resource and 3 ask to be regenerated (the K.1 row).

`A254` recorded the owner's words: "Note that XFA is rejected for now, but we will implement it later."

## What has not changed, and what blocks

Nothing in the three rows has decayed. What stands between "later" and a build is RFC 0009 section
4.3's list, re-checked on 2026-10-26:

- **The specification.** XFA 3.3 is Adobe's, hosted by the PDF Association; it answers 403 today, as
  on 2026-10-05, and nothing of it is in `doc/`. Principle 5 makes it the precondition.
- **The authoring line.** A dynamic form is schema-driven page generation: a layout engine that decides
  how many pages there are and where every field goes, which is the middle of the fourth exclusion.
- **FormCalc**, a second script language with no other home, and XFA's event model beside RFC 0008's.
- **The launch path**: `/NeedsRendering` asks for the regeneration when the document is first opened.
- **No admissible library**: `pdf-xfa` 1.2.0, the one Rust XFA stack, is AGPL-3.0-only or commercial,
  which `deny.toml` refuses.

## The question

When does "later" begin, and is its first step the specification in `doc/` and an RFC?

## Recommendation

**Not in the coming batches, and when it begins, begin with the document, not the code**: obtain XFA
3.3 into `doc/` (the PDF Association's member area, or Adobe's own copy), then one RFC round that
places a static form's template behaviours (calculations, validations, formatting — JavaScript ones
through `pdf-script`) apart from a dynamic form's layout, which needs its own amendment to the
authoring exclusion. A static form's half reaches the 4 hybrid files; the layout reaches the 3 that
show a notice.

**What the tree does meanwhile**: every window says the form is XFA and fills the `AcroForm`; the rows
stay `out-of-scope`.
