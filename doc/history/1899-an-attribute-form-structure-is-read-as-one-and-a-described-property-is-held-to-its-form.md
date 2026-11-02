# 1899 — an attribute-form structure is read as one, and a described property is held to its form

Session 1899, batch 130, slot 9 (robustness, "parsers"), in two sessions. ADR 2632; Q775 to the owner; ADR 2633
unused.

**What changed.** `pdf_model::xmp`'s reader takes a property, field or item element carrying property attributes
(not `xmlns`, `rdf:` or `xml:`, and no `rdf:resource`) as a structure whose fields are those attributes — the RDF/XML
Syntax Specification (Revised) section 7.2.21's empty property element, admitted by the XMP Specification of
September 2005 (p. 21). The row `metadata/properties-use-known-schemas` (ISO 19005-2 6.6.2.3.1) now also holds a
property an extension schema describes, in its own stream or the catalog's, to the form its value type fixes
(`judge_against_description`, `pdf_archive::properties_outside_their_description`); `prepare_properties` cuts what
that list names as it cuts the predefined half. With the reading right, `xmpMM:Ingredients` in `143424…`'s image
packet is undeterminable and dropped under `undeterminable = "discard"`, as `xmpMM:Manifest` already was.

**The premise that did not hold.** Not placement: a page-level packet inherits the catalog's schemas under 6.6.2.3.2
and the checker reads every packet. The converter wrote the `Bag Text` description itself, from a reader that took
each `<rdf:li stRef:filePath="…"/>` as empty text, and the checker had no described-form judgement to catch it.

**`T02-03_006`.** Its root states `/Count 1` and one kid the cross-reference places on another object. §7.7.3.2's
Table 30 and the Arlington model (`Count >= 0`) admit a document with no page, so the ground is A727's
`page-not-held` with zero pages carried. Q775 asks whether that is a zero-page archive (the statement page naming the
page not held) or a refusal by name; nothing is built, and the converter's refusal names the page not held.

**Measured.** HEAD's 2b output of `143424…` fails the branch's checker at object 553 `xmpMM:Ingredients` and at
nothing else at 2b, as veraPDF fails it; the branch's outputs pass veraPDF at 2b and 4. The plant (`Ingredients`
described `Bag ResourceRef`, in the stream and in the catalog's packet) passes. Census no-pack and with the pack
against HEAD's: no line moved, summary or per-document.

**Unfinished.** No converter-level test drives `prepare_properties` through a source packet whose own description
contradicts its value; the row's unit test and `143424…` cover the judgement and the reading.

**Gates.** `cargo test -p pdf-model` exit 0 (2369 passed), `-p pdf-archive` exit 0 (311), `-p pdf-transform --test
archive --test profiles` exit 0 (528); on the export `-p pdf-archive --test cross_check --test witnesses -- --ignored`
exit 0. Clippy `--no-deps -D warnings` on the three crates `--all-targets` exit 0. Census two runs exit 0. veraPDF 3
runs: branch 2b and 4 exit 0, HEAD 2b exit 1. `--bin cited` exit 0, `--bin quotations` exit 0. `cargo test -p
conformance --no-fail-fast` exit 101: `fuzz_workspace` (a `type1_write` target without its `doc/verify.md` line and
`seeds.sh` arm) and `records` (1892's record), both siblings' in progress.
