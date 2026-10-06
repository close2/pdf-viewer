# 1584 — A `post` table's names are read once per face, and a `/Differences` name is not searched for

Session 1374. Status: **accepted**. Supersedes nothing. Context: `doc/todo/49`'s fuzz-triage
bullet; trap 50; habit 63 (count before you build); ISO 32000-2 §9.6.5.4 and §9.10.2; the
OpenType specification's `post` table, versions 1.0 and 2.0 (cited by section, never quoted,
`doc/third-party-data.md`).
Code: `crates/pdf-font/src/post.rs` (new, `PostNames`), `crates/pdf-font/src/truetype.rs`
(`named_glyph`, `truetype_code_table`), `crates/pdf-font/src/loading.rs` (`program_characters`),
`crates/pdf-font/src/glyph_names.rs` (`glyph_name_of`).
Tests: `crates/pdf-font/tests/post_table_names.rs` (new), the moved `truetype` unit tests.

## 1. What was quadratic, and why

The OpenType `post` table, version 2.0, holds one 16-bit index per glyph and then a run of Pascal
strings with no offsets. An index below 258 names one of the standard Macintosh names, listed in
the version 1.0 section; an index of 258 or more names the string at that position less 258.
`read-fonts`' `Post::glyph_name` finds the n-th string by stepping over the n before it, on every
call, and its own documentation of `VarLenArray::get` says so. `post_glyph` searched glyph 0 upward
for a name, once per code that §9.6.5.4 sent to the `post` table. That is the square of the glyph
count over two per code. `program_characters` asked every glyph's name once, which is the same
square once per face.

## 2. The decision

`PostNames` reads the table's header when it is made, and nothing else until something asks. The
first question resolves every glyph's name in one pass over the strings. The first name-to-glyph
question then builds one map from that. Where two glyphs share a name the lower one is kept, which
is what the search answered. A non-ASCII string or an index past the strings names nothing, as
before. Versions other than 1.0 and 2.0 name nothing, as `read-fonts` answers. The 258 standard
names are `read-fonts`' static `DEFAULT_GLYPH_NAMES`.

The map is a `HashMap` under the standard library's random keys. The names are the document's, so
a fixed hash would let a file fill it with colliding names. A `BTreeMap` was measured first. It cost
about 890 instructions a name to build against about 280 for the hash map. On `S2.pdf`'s two
2194-glyph subsets that made the face cost more than the search did.

## 3. Measured (callgrind, `callgrind_interpret` page 1, each arm its own binary, `md5sum` apart)

| document | before | after |
|---|---|---|
| `slow-unit-0c5ebcab…` (5125 glyphs), whole page | 14 487.3 M | 131.6 M |
| `issue14297.pdf`, `LoadedFont::load_simple` | 338.0 M | 23.0 M |
| `bug1727053.pdf`, the same | 72.2 M | 22.9 M |
| `bug1200096.pdf`, the same | 4.97 M | 2.29 M |
| `issue4800.pdf`, the same | 3.98 M | 3.17 M |
| `S2.pdf`, the same | 5.39 M | 6.63 M |

`S2.pdf` is the cost, stated: its subsets send few names to `post`, so resolving every name costs
1.2 M instructions more than the few searches did, about 3% of its page. A count of lookups before
building the map would win it back at the price of a second code path, which this decision does
not take. The regression test's 5125-glyph case took 6.98 s on the old code and 0.02 s for both
cases on the new. Its ceiling is 1 s.

## 4. The audit, counted before anything else was built

Callgrind over page 1 of 58 pdf.js documents: the 25 with the largest format 2.0 `post` tables
and 35 drawn at random from those with a `/Differences` array, two of them in both. The self cost
of each `pdf-font`, `read-fonts` and `skrifa` function:

- **`/Differences` names: fixed.** `glyph_name_of` searched the 391 standard strings and four
  encodings' tables for each name, about 1400 comparisons for a name in none, to avoid one small
  allocation. That was 28% of `issue4550.pdf`'s page. `encoding_names` is now 1.28 → 0.11 M there
  and 2.35 → 0.27 M on `issue215.pdf`. The name is now always owned.
- **`cmap` format 4: no shape.** `read-fonts` binary-searches its segments.
- **`loca`, `hmtx`: no shape.** `skrifa` indexes both directly, and `sfnt.rs` restates `hmtx`
  in one pass.
- **CFF charset and encoding: linear, left.** `read-fonts`' `Encoding::map` calls
  `Charset::glyph_id` once per code, and that walks the charset. That is 256 times the glyph count
  once per face, not a square. It cost 2.1 M (5.2%) on `issue215.pdf` and nothing visible on the
  other 57.
- **`/W` widths: a different shape, priced in `doc/todo/49`.** `composite_widths` was the largest
  font cost of the audit, 29.9 M (28.6%) of `issue16553.pdf`'s page. That is one tree entry per
  CID of a dense list, not a search.
