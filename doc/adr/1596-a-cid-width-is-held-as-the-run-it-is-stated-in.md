# 1596 — A CID's width is held as the run it is stated in, a charset is inverted once, and a face asks `post` a few names before it builds a map

Session 1380. Status: **accepted**. Amends ADR 1584 section 3's last paragraph, which priced the
`S2.pdf` regression and declined the second code path that wins it back. Context: `CLAUDE.md`
principle 3 (explicit memory budgets); `doc/todo/49`'s `/W` bullet; ISO 32000-2 §9.7.4.3; the CFF
specification (Technical Note 5176), cited and not quoted.
Code: `crates/pdf-font/src/runs.rs` (new), `crates/pdf-font/src/metrics.rs` (`composite_widths`,
`composite_vertical_metrics`, `Vertical`), `crates/pdf-font/src/loading.rs` (the `widths` fields),
`crates/pdf-font/src/cff.rs` (`CodeToGlyph::read`), `crates/pdf-font/src/post.rs`.
Tests: `runs::tests` (seven), `metrics::composite_widths_tests` (three); `fuzz/seed_widths.py`.

## 1. What was unbounded

§9.7.4.3's second form, `cfirst clast w`, gives one width "for all CIDs in the range c first to c
last". `composite_widths` wrote one `BTreeMap` entry per CID of each range, capped at 65 536 a
range, and `/W2` did the same. Nothing capped the number of ranges, so an array of n ranges asked
for up to 65 537 n entries from about 20 n bytes of file. That is arithmetic from the code: the
new test's array, 4097 ranges in about 70 KiB, is about 268 million entries by the old reading.
The cap also cut a legitimate range short, since §9.7.2 leaves the largest CID to implementation
limits and Annex C's 65 535 is a recommendation of previous versions.

## 2. The decision

`Runs<T>` holds disjoint runs sorted by first key, each pointing into one flat value array. A range
is one run with one value, and `c [w1 w2 …]` is one run with consecutive values, split where an
element is not a number. A lookup is a binary search. `RunsBuilder` keeps the clause's rule, "the
first specification is the one that shall be used": a statement claims only the CIDs no earlier
statement covered. The covered set is a map of merged intervals, so the work over an array is its
statement count times a logarithm whatever the overlaps. A table costs at most two runs per
statement and one value per number in the array. That is bounded by the array the parser already
holds, so **there is no budget and no refusal**: a budget would bound the array, which is the
parser's to bound. The cap is gone and a range spans what it states. A range whose last CID is
below its first now names no CID. `/W` gave its first CID the width and `/W2` gave nothing, and the
clause's "range c first to c last" is empty there. Nested arrays are read in place, not cloned.
A simple font's `/Widths` uses the same type through `Runs::from_map`.

## 3. The two leftovers of ADR 1584

**The CFF charset.** `read-fonts`' `Charset::glyph_id` walks a custom charset from its start, and
`Encoding::map` asks it once per code under a predefined encoding. `CodeToGlyph::read` now inverts
the charset once, in the pass it already made for names. The inverse holds only the 391 standard
string identifiers, which are all a predefined encoding can name, and the lowest glyph for each,
which is what the walk returned. Predefined charsets are left to `read-fonts`, which answers them
from static tables. Custom encodings are left to `Encoding::map`, which walks only for supplements.

**`post` on `S2.pdf`.** `PostNames::glyph` searches the resolved names for the first 64 asks and
builds the map on the 65th (`SEARCHES_BEFORE_MAP`). A search costs a few instructions a glyph and
the map about 570 a glyph. The worst case, 256 asks, is 64 searches plus the map, linear in the
glyph count. `S2.pdf`'s two faces build no map under this rule: no map initialisation is in the
profile.

## 4. Measured (callgrind, `callgrind_interpret` page 1, `md5sum` apart, both arms behind the lock)

| document | quantity | before | after |
|---|---|---|---|
| `issue16553.pdf` | page | 104.39 M | 62.27 M |
| | `composite_widths`, inclusive | 44.84 M | 5.71 M |
| `issue215.pdf` | page | 9.29 M | 8.71 M |
| | `CodeToGlyph::read` | 2.95 M | 2.36 M (`Charset::glyph_id` 0.59 M → none) |
| `S2.pdf` | page | 42.15 M | 40.91 M |
| | `LoadedFont::load` | 6.63 M | 5.48 M |

What remains of `issue16553.pdf`'s widths is the lexer and the object copies of `get_key`, which
are `pdf-syntax`'s. A first draft of the CFF inverse used a `BTreeMap` and saved 0.13 M rather
than 0.58 M, which is why it is an array.
