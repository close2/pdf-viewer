# 2461 — A CJK descendant is given a descriptor of its own where it states none, or shares one at other widths

Status: implemented. Session 1813, the archive slot of batch one hundred and twenty-one. Context:
ADR 2302 (the CJK pack's route), ADR 2179 (simple fonts sharing a descriptor keep their own
programs through descriptor copies), ADR 2460 (the census with the pack).

## The two refusals

With the pack installed, two of round 1804's thirteen documents still refused at
`fonts/font-programs-embedded`, each for a reason the pack route had not built:

- `90ms_rksj_h_sample.pdf`'s descendant `CIDFont` states no `/FontDescriptor` at all, which
  ISO 32000-2 Table 117 requires. The route read the entry as a reference and refused with the
  sentence for a descriptor *written inside the descendant* — a sentence about a condition the
  file did not meet (trap 11).
- `issue13343.pdf`'s three descendants name one descriptor at three `/DW`s (1180, 1000, 960).
  One descriptor holds one program and a program states one advance per glyph (§9.2.4), so the
  route merged the second font's widths into the first's subset, found CID 843 at two advances and
  refused under the fence.

## The decision

`pack::home_of` decides where a descendant's subset goes. It goes into the descriptor the
descendant names where that descriptor is an object no program was yet written into, or one this
route wrote for the same collection at widths that agree — one subset of every CID the fonts
reaching it show, as before. Otherwise the descendant is given a **descriptor object of its own**,
pointed at from its `/FontDescriptor`, holding a subset of its own:

- a descriptor written inside the descendant, or one already holding another font's subset, is
  copied: its entries are the producer's, and `Embedding::write_into` drops the `/CIDSet` and
  `/CharSet` that described an absent program, as for every route;
- no descriptor at all is **built**, each Table 120 entry the table requires read off the
  programs: `/Flags` Table 121's `Symbolic`, `/FontBBox` every glyph of the subset, `/Ascent` and
  `/Descent` the subset's glyphs but `.notdef`, `/CapHeight` the top of the collection's `H` in
  the pack program (found through the collection's own `Adobe-*-UCS2` table), `/ItalicAngle` the
  program's `post` table's, `/StemV` 0, Table 120's "unknown". A subset reaching no point below
  the baseline is refused by name, since Table 120's `/Descent` shall be negative.

Two `CIDFont`s at different widths are two fonts; a descriptor copy for each is the same
construction ADR 2179 gives simple fonts. The refusal for a shared descriptor (another collection
or another route) is gone, because the same copy answers it.

## Consequences

- Both documents convert under `as-if-printed` at 2b and 4 with the pack; under `as-shown` they
  stop on `machine-face`. `doc/history/1813` has the census and veraPDF's verdicts.
- `tests/archive/cjk_pack.rs` holds a fixture for each shape, run by CI's `cjk-pack` job with the
  other pack tests; neither runs without a pack, as no pack test does.
- Not built: a descendant whose `/FontDescriptor` is neither absent nor a dictionary, refused as
  `NOT_A_DESCRIPTOR`; no corpus document has one.
