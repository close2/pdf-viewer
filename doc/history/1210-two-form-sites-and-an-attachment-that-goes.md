# 1210 — Two form sites and an attachment that goes

The corpus-named round, on `doc/todo/66`'s forms and embedded-files families: the three sites
`as-if-printed` still named at PDF/A-2b, 2u and 2a.

- **`forms/need-appearances-absent-or-false`** (ADR 1257). §12.7.3's Table 224 makes removing the
  flag a *claim* that appearance streams have been provided for every visible widget, so the
  construction beside it is what makes the claim true. `pdf_archive::visible_widgets` is the
  population — visible by §12.5.3's Table 167, no `/N` by §12.5.5's Table 170 — so a widget whose
  producer already wrote a stream keeps the producer's bytes. A button field's widget and any
  appearance `pdf_model::appearance` cannot complete leave the site refused with the **field**
  named in `Conversion::unconstructed`; `construct = false` is the other answer and costs
  `Loss::FieldAppearances`, routed through a new `Conditional::PerConfiguration`.
- **`forms/no-xfa-key`** (ADR 1257). `Loss::XfaForm`, on §K.2's requirement that the interactive
  form dictionary agree with the resource. The dynamic predicate is the **document's own**
  sentence: §7.7.2's Table 29 makes `/NeedsRendering` true the claim that the pages are
  regenerated when the file is first opened. Annex K states no test and says whether an XFA
  schema's pages are materialised is implementation dependent, so the pages settle nothing either.
  `dynamic = "stop"` is the default, and the limit is written down: a dynamic form whose producer
  wrote no `/NeedsRendering` is indistinguishable from a static one.
- **`embedded-files/embedded-file-is-itself-pdfa`** and its plain-profile twin (ADR 1258).
  `Loss::EmbeddedFile`: every reference goes — the `/EmbeddedFiles` entry and every `/AF` place —
  and the serializer drops what nothing reaches. §7.7.4's Table 31 is why both go. Refused by name
  where a `/FS` still reaches the specification, and where the name tree writes it in place.

Beside them:

- ADR 0965's sentence that `/NeedsRendering` is lossless *because `/XFA` is refused* stopped being
  true; the removal is still lossless on Table 29's own default, and the comment now says that.
- A `derive` whose declared tool was run and did not answer used to fall through to
  `REFUSED_BY_NAME` by accident. With a `Loses` row behind the site it would have fallen through
  to an unauthorised loss nobody asked for, so `configured` now refuses such a site outright, at
  every `derive` site rather than at these two.
- `editions::SHIFTS` gained §12.7.3: ISO 32000-1:2008 numbers the interactive form dictionary
  12.7.2, and its own 12.7.3 is *Field Dictionaries*, so the citation resolves and names something
  else.

`as-if-printed` now answers every site it names at 2b, 2u and 2a with a remedy this version carries
out. Part 4 still owes it five, and `only-metadata-loss` two to three — the contract's fourth item,
`original = "attach"` and the two `/Info` `preserve`s, was not reached.
