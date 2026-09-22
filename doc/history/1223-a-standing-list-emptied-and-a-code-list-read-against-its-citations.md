# 1223 — A standing list emptied, and a `code` list read against its citations

## What moved

No ledger status. `tests/names.rs` is a zero gate now: `STANDING` (44 lines) is gone and any
finding fails with its file, line and sentence (`tools/state.sh names`). `--bin cited`'s two rungs
went from 573 and 299 to 138 and 37 (`tools/state.sh cited`); 707 files were added to the `code`
or `test` array of the row whose clause they read, and no note or status changed.

## Findings

**Of the 44 names, most were renames the prose had not followed.** `black_point` is
`black_point_under`, `program_shortfall` is `repair_shortfall`, `FontCache::bind` is `Kept::bind`,
`Open::current` is `Open::shown_page`, `Answer::Form` is `Answer::Fields`. Some were wrong in
substance as well as in name. `MAX_GROUP_BLIT_PAGES` described a bound per page that is really
`MAX_GROUP_BLIT_PIXELS`, an absolute one. `image.rs` said §10.5's transfer copies the raster, but
ADR 1125 moved it onto the mark. `bring_up.rs` said the instance cannot be supplied, but
`for_surface_with_instance` takes one. `cited.rs` documented three rungs where the code has two.
None was a macro-generated item, so the gate did not need teaching.

**`--bin cited`'s leftovers are checkers and mentions, and it is not a gate.** `pdf-archive`
cites clauses it checks rather than implements, which is 1218's reading. Protocol and ABI files
carry the data a clause defines. Every leftover is listed in the report. Two citations were
wrong: `pdf-font`'s subset prefix cited ISO 32000-1's §9.6.4 for ISO 32000-2's §9.9.2, and
`raster-scene`'s `cost.rs` used `§5` for a section of its own brief.

**`contradicted_frame` was a JPEG check (§7.4.8), not a JPEG 2000 one**, and it re-ran the filter
chain on every `Do`, cached ones included. The decode's grid now travels as `Parts::contradiction`.
Over the 62 corpus documents holding a `DCTDecode` image the reports match before and after.

**Three spellings of one date.** `merge.rs`'s `as_xmp_date` now calls
`pdf_model::xmp::spelled_date`, which the archive converter already used. `pdf_syntax::Date`'s
`Display` writes the same string, and folding `spelled_date` into it is a later round's edit.

## Handed over

- `--bin cited`'s 189 pairs with no row include roughly 448 raster citations written `§N` that
  mean sections of `raster/doc/*.md`. The brief's rule says those should read "section N".
- A checker citing the clause it checks is re-listed every round (`pdf-archive`'s 50 pairs).
