# 2439 — The eighteen departures read for a reason that no longer holds

Session 1802, a "clause" round of batch one hundred and twenty, 2026-10-27.
Status: accepted — one reason found false and its departure ended (ADR 2438); one found half false
and left standing on its other half, with the build that ends it named; three put to the owner in
`doc/questions/Q668`. Context: ADR 1622 (the last re-read, whose rule is kept: a departure is from a
`shall`), ADR 1165, ADR 2370, `cargo run -p conformance --bin departures` (each row's reading list).

## 1. The method

Each row's note and the ADR its first sentence names, read for one of three answers: the reason
still holds (a sentence of the clause, or a figure), the reason is false today (the evidence in the
tree), or the question is a scope or cost judgement that is the owner's. The population is the
command's, `status = "departed"` over `doc/conformance/ledger.toml`, and was eighteen.

## 2. The rows

| row | answer | the ground |
|---|---|---|
| §7.4.2 | **false — ended** | ADR 0036's "refusing loses the whole stream" took an error to be a refusal; ADR 0343's damaged decode carries an error beside the bytes, and ADR 0836 already uses it for RFC 1950's. ADR 2438 |
| §7.4.8 | owner's call | Table 13's default of 1 is the processor's `shall`; the exception rests on a `libjpeg` convention no clause states, at one page, `issue11931.pdf`. Q668 (1) |
| §7.5.5 | holds | Table 15's `/Size` sentence obeyed per section loses page one of a 66th document and rebuilds 65 (ADR 2346) |
| §8.7.4.5.7 | holds | no backend holds a colour space to convert a patch interior in (ADR 0028); `mesh::PATCH_STEPS` is still those patches' fineness, and no document takes the branch. Ending it is a colour grid through `pdf_render::SurfacePatch` to every backend |
| §8.7.4.5.8 | holds | §8.7.4.5.7's, one `tessellate` |
| §10.7.4 | holds | four `shall`s, each with its ADR (1082, 1560, 0738/2331, 0025, 2330); nothing since them has moved a figure |
| §10.7.5 | holds | grid-fitting not performed, 0.0039 of a device pixel from the requested width (ADR 0848) |
| §11.5.3 | holds | `MAX_PRESSES` is still `RENDERINGS_OF_ONE_PROFILE` (7) twice over, against a deepest page of one press (ADR 1254) |
| §11.6.6 | holds | the same bound |
| §11.7.2 | holds | the same bound |
| §11.7.5.3 | holds | a press sampled from its output intent converts a paint through that profile's own `B2A` (ADRs 0796, 1242) |
| §12.5.2 | holds | both shortfalls still return no appearance — `write_retyped_appearance` where the text cannot be laid out, `write_added_appearance` where `fsType` forbids embedding — and ADR 2170 priced the third way |
| §12.5.5 | holds | Table 166's `/ca` and `/CA` rows and §12.5.5 are three `shall`s in conflict, all still the standard's |
| §12.5.6.23 | holds; one half the owner's | ADR 1363's calculator function stands; a `JPXDecode` component past Table 87's sixteen bits can be removed without trace only through a lossless JPEG 2000 writer. Q668 (3) |
| §12.6.4.6 | owner's call | whether a reader may set a level at which a document launches a *program* is principle 3 against the reader's levels. Q668 (2) |
| §12.7.4.1 | **half false; stands** | ADR 1198 §5 priced a cycle detector at "a set per field", and a two-pointer detector needs none; but the bound also caps a deep acyclic chain walked once per widget, widgets × depth, which principle 3 still asks a budget of. Ending it is each field's inherited entries resolved once per document, over the six walks `MAX_FIELD_ANCESTRY` and its copies bound |
| §12.7.8.3.3 | holds | `ViewState::save` still writes no appended page (`Written::unwritten_pages`), and no field dictionary is made under a new name |
| §12.10.2 | holds | the owner's answer A271 decided it (ADR 1918) |

## 3. Consequences

The hypothesis — two or three reasons no longer hold, one is built — held at one and a half. The
cheapest false one was §7.4.2, because the carrier already existed; §12.7.4.1's other half is a
build of its own and is named above rather than started.
