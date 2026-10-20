# 1607 — A structure field is described in its type, the identification namespace is `http`, and a 3D annotation outside PDF/A-4e is the subtype rows' loss

Slot 1 of batch ninety-six, 2026-10-20, an archive round. ADRs 2050, 2051; no question.

**Premises and hypotheses.** The census premise held: `6-6-2-3-3-t03-fail-b` was the last 2b
`not-built-yet` site with a document. The requirement cites section 6.6.2.3.1, not 6.6.2.3.3 (the
brief's); veraPDF reports it there too. "The description is a `supply`" held only as an option: the
default is ADR 1245's statement one level down. The namespace hypothesis held: PDF Association erratum
#123 (ISO-approved) corrects part 4's `https` to `http`; `pdf_archive::errata` carried it on a part 2
row. The URI is in `prepare.rs` and `pdf-archive`, not `pdf_model::xmp`. "Part 4's next sites are the
structure-tree root family" did not hold: the veraPDF census ranks `3D`/`RichMedia` outside 4e (19)
and `FileAttachment` outside 4f (13) at 4, the two `Lang` rows (25, 22) at 2a.

**Built.** ADR 2050: `pdf_archive::undescribed_fields` (the row's reading as a population),
`pdf_model::xmp::describe_fields` (a Table 6 item inserted into the producer's own type description),
`Rewrite::ExtensionFieldDescribed`, a sixth packet editor, `Stated`, the report's
`described_extension_fields`; another-namespace, structured-value and not-once-described fields
refused by name. ADR 2051: the writer writes `http` for both parts, the validator accepts both on the
erratum's note, #123 moved to the four part 4 rows with a test holding every erratum to a part 4 row.
ADR 2051 section 4: `annotations/three-dimensional-only-in-engineering-files` is
`Loss::ForbiddenAnnotation` over `annotations_outside_the_engineering_flavour` chained into the one
removal, and preservable by page; `keep-everything` and `as-shown` answer it `preserve`. The no-op
`HTP` `preserve` left `keep-everything.toml` with a sentence.
**Pixels.** `6-6-2-3-3-t03-fail-b` at 2b; `6-2-2-t02-fail-a`, `6-2-3-t01-fail-b` and the first at 4:
every page byte-identical at 72 and 720 dpi to the input's; veraPDF 1.30.3 failing before (6.6.2.3.1
test 2; 6.7.3-1 at 4), compliant after. `6-3-1-t01-fail-d` at 4 under `as-shown`: byte-identical, the
invader poster looked at; with `--authorise forbidden-annotation` the poster goes, as the loss says.

**Gates.** rustfmt clean per file; `cargo fmt --check` and `RUSTFLAGS=-D warnings clippy
--all-targets` for `pdf-model`, `pdf-archive`, `pdf-transform` exit 0; their nextest 2959 passed, 0
failed (after `cargo build -p pdf-sandbox --bins`, trap 10). `cargo test -p conformance` 442 passed, 1
failed: `state_sections`, the moved `quorra/renderer.rs` (slot 2's). `tools/state.sh --round 1607
archive` exit 0, 510 s: `--remedy-sites` 2b 24 of 68 (25 of 69), 4 22 of 60 (23), 4f 22 of 59;
`as-shown` at 4 2 answers not carried out (3); validator over the veraPDF corpus `missed` 0, `over` 0
every target; `archive_corpus` 2b 492/119 (491/120), 4 233/126 (231/128); `cross_check` 0
disagreements. veraPDF census `--tree 6` exit 0: no `not-built-yet` site with a document at 2b or 2u.
`t3-foreign_corpus` under `--clock` exit 0, 196 s, 0 panicked.

**Unfinished.** `FileAttachment` outside 4f (its file kept in the name tree); the 2a `Lang` pair; a
`supply` of an operator's field description. 4 700 s.
