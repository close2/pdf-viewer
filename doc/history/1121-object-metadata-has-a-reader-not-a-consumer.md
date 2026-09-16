# 1121 — §14.6.2's object metadata has a reader, and what it lacks is a surface

2026-09-16. Files: `crates/pdf-model/examples/object_metadata_census.rs` (new),
`crates/pdf-model/tests/xmp.rs` (one fixture), `doc/conformance/ledger.toml` (§14.6, §14.6.2,
§14.3.2's notes), this file. Every other path in `git status` is a sibling's. No pixel moved.

## The contract

§14.6.2's row was `partial` on a note whose list of what a property list is read for ended "and
nothing else" while the artifact and associated-file readers stood — a list of what is read is
maintained by nobody (ADR 1023). Correct it to what is, and settle the genuine residue: is
§14.3.2's object-level `/Metadata` (Table 348, on a form XObject, image or annotation) read where
the clause requires?

## The census (measure first, trap 8)

`examples/object_metadata_census` walks every object's graph. Over the 974: catalog 319 stated /
318 read back, image 3/3, other stream 55/55, **form XObject 0**, annotation 0. Over the 65 944
crawl: catalog 38 927, image 143 503, other 78 060, page 907, **form XObject 0**, annotation 0 —
and `Xmp::read` reads back all but ~2% (the undecodable, the class of the one fuzzed catalog
stream). So across 66 918 documents **no file states a `/Metadata` on a form XObject**, the carrier
§14.3.2's property-list sentence points at, and the reader reads back every carrier that appears.

## The finding

`Xmp::read` already takes a *dictionary*, not a document, so it is general over Table 348's
carriers — it is invoked only for the catalog because no in-scope clause *consumes* an object-level
packet. Object-level metadata is interchange (§14.3.1) and decides nothing §6.3.2.2 makes a
rendering processor owe. A property-list `/Metadata` is one call from being read — `Xmp::read` over
the dictionary `content::marked::property_list` already returns — so what is missing is a surface,
not a reader, exactly what §14.3.2's own row records everywhere. A documented non-need, not a debt.

## The fixture (trap 13) and the rows

`tests/xmp.rs::a_metadata_stream_on_a_form_xobject_reads_back` plants a `/Metadata` packet on a form
XObject — the carrier the corpus never states — and confirms `Xmp::read` returns its `dc:title`.
§14.6.2 → **implemented**: both forms read, every in-scope consumer enumerated by the `test` array
rather than a hand-list, object metadata a documented non-need. §14.6 → **implemented** (its sole
stated debt was §14.6.2's). §14.3.2's note gains the generality measurement. Gates: tier 1 clean;
`xmp`, `dates`, `corpus`, `raster_golden` unmoved.
