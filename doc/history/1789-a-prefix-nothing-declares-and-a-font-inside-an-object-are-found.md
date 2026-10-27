# 1789 — A prefix nothing declares and a font written inside an object are found

Slot 2 of batch one hundred and nineteen, 2026-10-26, the archive slot; ADRs 2412 and 2413; Q655
unused. I worked in `scratchpad/r1789/`. No ledger status moves.
**Built (ADR 2412).** `pdf_model::xmp::Xmp::undeclared_prefixes`: every prefix an element or attribute
uses that no binding in scope declares (Namespaces in XML section 5, an empty declaration by its
section 3), a reading of the same walk as `rdf_elements`; `parse` is unchanged. ISO 16684-1 section
7.1's well-formed RDF carries it, since RDF/XML takes no document that fails Namespaces in XML, so
`metadata/xmp-packets-well-formed` — both parts — reports each prefix, and `PacketFault::UndeclaredPrefix`
sends the packet down ADR 1245's fresh-packet route under the existing metadata loss. **Premise**:
the hypothesis that the data-model row binds part 4 did not hold — part 4 asks for well-formedness,
not conformance, so the row stays part 2's; the fault is well-formedness.
**Built (ADR 2413).** The font population reaches a `/Type /Font` dictionary written directly
inside an object the table names (a direct descendant, a font in a `/Font` resource), walked once
per examination (`Examination::direct_fonts`). Such a font is reported at no object, its holder
named in words, because the converter edits a finding's object as the font; the CMap list keeps
own objects, and `cid_to_gid_maps` refuses a direct descendant with a sentence of its own.
**Premise**: `Test-plusminus` held; `issue12823` did not — its `/DescendantFonts` is `[ null ]`,
under the `Unchecked` row `fonts/font-programs-conform-to-their-own-specifications`, left owed.
**Census** (converted/conforming/refused/unreadable), HEAD → this: pdf.js `as-if-printed` 2b
790/2/164/11 → 789/2/165/11, 4 797/0/159/11 → 795/0/161/11; `as-shown` 2b 765/2/189/11 →
764/2/190/11, 4 773/0/183/11 → 771/0/185/11; `doc/corpora` one more refused in each of the four;
the veraPDF corpus lines unchanged. Leaving the converted lists: Test-plusminus (all four),
issue20453 (part 4), breast+cancer+immunotherapy (all four; a `<dc:creator/>` under `rdf:RDF`, no
`xmlns:dc`); none joins. Over the 97 documents a qpdf scan finds holding a direct font or an
undeclared prefix, HEAD's and this converter's reports differ on those three, on hesr13613 (a second
reason) and on bug854315 (refused both times, over its signature).
**veraPDF 1.30.3** on HEAD's outputs of the three, both profiles: all FAIL, on 6.2.11.3.2-1 /
6.2.10.3.2-1 and on 6.6.2.1-4, -5 / 6.7.2.1-4, -5 beside the identification rule; none is freed.
**Proof**: the direct-font test planted (direct half dropped) fails; the prefix tests name `dc`.
**Gates.** `rustfmt --check --edition 2024` on my files exit 0; in an export of HEAD and my paths,
`cargo fmt --check` and `RUSTFLAGS="-D warnings" cargo clippy --all-targets` on pdf-model,
pdf-archive and pdf-transform exit 0, rustdoc `-D warnings --document-private-items` on pdf-archive
and pdf-model exit 0. nextest: pdf-archive 299, pdf-transform 706, pdf-model 2260 passed, 0 failed.
Census `--tree 6` exit 0 before and after; cross-check exit 0, 13 560 compared, 0 disagreements
before and after; `pdf-model --test xmp` exit 0; `tools/state.sh --round 1789 archive` exit 0
(validator missed 0, over 0 at six targets; every conforming input still conforming). The arms:
`cargo tree -i pdf-archive` reaches no arm binary and the drawing paths reach `xmp.rs` only
through `unescape`, unchanged, so against HEAD's six arms nothing can move. `cargo test -p
conformance --no-fail-fast` exit 0; a later run's one red is 1792's record, not mine. 7045 s.
