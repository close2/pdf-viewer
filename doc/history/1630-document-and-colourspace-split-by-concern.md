# 1630 — `Document` and `ColourSpace` split by concern

Session 1630, slot 8 of batch ninety-eight, a docs refactoring round (Sonnet 5.5); 2026-10-20. ADRs
2096 and 2097. The owner's words: "Refactor god classes,..." and "Do not fix any bugs." None was fixed.

**Built.** `doc/todo/74` entry 8. `crates/pdf-syntax/src/document.rs` (3 364 to 1 052 lines, about 600 of
them tests) keeps `Document`, the entry points, the bounds and the public result types; nine modules
under `document/` hold `impl Document` by concern (`objects`, `parsing`, `repair`, `encryption`,
`object_streams`, `reports`, `decoding`, `images`, `sources`) and `cache` holds the decoded-stream cache.
`crates/pdf-colour/src/colour.rs` (7 613 to 1 897 lines, 1 460 of them tests) keeps `ColourSpace`,
`Tints`, `Separations`, `Reading` and the tests; fifteen modules under `colour/`: the `impl ColourSpace`
as `parsing`, `separation`, `queries`, `to_rgb`, `to_cmyk`, and the free items by clause as
`compositing`, `spots`, `grey_route`, `rgb_route`, `conversion`, `device_cmyk`, `press`, `ink_search`,
`cie`, `entries`.

**The premise that did not hold.** The entry priced `ColourSpace` as "45 methods in a 7 515-line file":
the method block is 1 740 lines and the other 4 000 are free types of other clauses (presses, ink
search, CIE arithmetic, the compositing key), so the colour split is mostly by clause, not by family.

**The proof that nothing moved.** Line multiset (visibility words removed, whitespace collapsed) of
each old file against its root and modules: removed-only 16 lines for `document.rs` and 37 for
`colour.rs`, all import lists, doc links re-qualified (habit 126), the tests' `super::cmyk(` kin
re-pointed, and signatures `rustfmt` wrapped; added-only are module docs, `use`/`mod` lines and the
`impl` wrappers. Six arms of an export of the tree against HEAD's by page: 0 missing, 0 differing.

**Ledger.** Twenty-two `document.rs` rows and forty-four `colour.rs` rows name the modules that cite
their clause (habit 96), read by `§<clause>` in each file. Teaching diagrams, `doc/crate-map.md`,
`doc/todo/74`, three doc paths in `shading.rs` and `pdf-model/tests/oracle.rs` follow.

**Found, not fixed.** No bug. `pdf-syntax` and `pdf-colour` have rustdoc errors under `-D warnings`
(links to private items, `[t]`-style brackets) before and after; neither is on ADR 1938's list.

**Unfinished.** Entries 4 and 9 to 11; the roots' test lines; `ink_search`, `press`, `spots` above 600.

**Gates.** `cargo fmt --check -p pdf-syntax -p pdf-colour` and `RUSTFLAGS=-D warnings cargo clippy -p
pdf-syntax -p pdf-colour --all-targets`: 0. `cargo test -p pdf-syntax -p pdf-colour`: 0, 424 passed, 1 ignored. `cargo check
--workspace --all-targets`: 0. `cargo test -p conformance --no-fail-fast`: 0, 443 passed. Six arms,
968 pages at 1x and 965 at 4x on each of cpu, gpu and compute: 0 missing, 0 differing; one lock hold of
2 150 s after 1 000 s queued, peak 3.72 GiB. Duration: about 5 400 s.
