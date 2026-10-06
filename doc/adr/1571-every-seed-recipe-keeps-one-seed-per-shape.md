# 1571 — Every seed recipe over the documents keeps one seed per shape, proved on a sample

Session 1368. Status: **accepted**. Extends ADR 1559 section 2 from `jpeg_bands` to every recipe
that reads the documents; amends its section 1 in one respect (the build). Context: traps 107 and
111; ADRs 0742, 0747, 1423, 1439, 1507, 1559; `fuzz/seeds.sh`, `fuzz/seed_shape.py` (new),
`fuzz/seed_page.py`, `fuzz/seed_streams.py`, `fuzz/seed_codecs.py` (new), `fuzz/seed_der.py`,
`fuzz/seed_cms.py`, `fuzz/seed_revocation.py`, `fuzz/seed_x509.py`, `doc/verify.md`'s fuzz lines.

## 1. The rule

`documents` is 90 763 files and 126 GB. Written whole, a recipe over it wrote tens of thousands of
seeds and gigabytes into the main checkout through the corpus link, and a campaign loads all of it
before its first mutation. What a target's code does with an input is decided by a few properties
of it, so each seeder now states per target the **shape** its target branches on, beside the
reason, and keeps the smallest seed of each (`seed_shape.Smallest`). The shapes, in brief:

- `xmp`: the bound namespaces, the RDF forms, the accessors' properties (empty or not, a date's
  form), element and attribute counts' orders, lexical forms; a packet under 64 bytes is its own.
- `sfnt`: version tag, table set, `indexToLocFormat`, glyph count's order, any composite glyph.
- `cmap`: `/CMapType`, `/WMode`, the codespace ranges' byte lengths, the operators, a `bfrange`
  array, the count of codes' order, lexical forms. `ccitt`: coding, columns, flags, the order of
  `/Rows`, whether damaged rows are concealed. `crypt`: `/V`, `/R`, `/Length`, the `/CFM`s and
  which `/StmF`, `/StrF`, `/EFF` name, `/EncryptMetadata`, the revision 6 entries, lexical forms.
  `variable_text`: the `/DA` operators with operand kinds and `/Helv` named, the value's length
  order, breaks, high bytes, byte-order mark and character classes.
- `jbig2`, `jpx` (`seed_codecs.py`, after `image_codec_seeds` writes into scratch): segment types
  with their procedure flags, referred-to counts and data-length orders; box set, `ihdr`, `colr`,
  and the main header's `SIZ`, `COD`, `QCD` and other markers; each with the seed length's order.
- `cms`, `x509`, `revocation`: `seed_der.shape`, the set of object identifiers anywhere in the
  structure and the length form; a revocation list adds its kind and its length's order.
- `object`: **objects, not documents** — its target stops at a document's first `obj` keyword, so
  every byte after `%PDF-1.x 1 0` was never read. An object body's grammar forms, length order,
  lexical forms and kind-naming keys. `lexer`: the lexical forms a document uses.
- `document`, `serialize`, `linearize`, `page`: the file's structure (version, updates, table or
  stream or hybrid, object streams, encryption, Annex F, `%%EOF`, filters, `/Type` and `/Subtype`
  sets); `linearize` adds the page count's order and `page` the constructs, colour spaces, blend
  modes, shading, function and pattern types and `/S` names the raw bytes state.

A seeder reads a document only if its memory map holds the name its target needs (`find`, never
`in`, which a map answers per byte). Over the first 400 pdf.js documents the needle walk finds
exactly the seeds the old scan over every object found, for all six stream targets and the three
DER ones. `--every` writes the population whole and is what a shape is proved against.

## 2. The proof: the old recipe and the new over 300 documents

For each target 300 documents were drawn (seed 1368) from those naming its needle, or, for the
whole-document targets, from the 47 523 under 256 KiB; each recipe ran both ways and `INITED cov`
was read from a `-runs=0` pass under the target's line's limits. Each shape was refined, by merging
the old seeds into the new set and reading what the gainers had that their shape's seed lacked,
until the loss sat inside `check`'s own 2% margin:

| target | whole: seeds, bytes, cov | one per shape: seeds, bytes, cov |
|---|---|---|
| `xmp` | 1664, 7.8 MB, 740 | 479, 4.3 MB, 740 |
| `sfnt` | 1118, 57 MB, 168 | 270, 14 MB, 168 |
| `cmap` | 40, 211 kB, 330 | 25, 193 kB, 330 |
| `ccitt` | 7988, 142 MB, 158 | 1454, 17 MB, 155 |
| `crypt` | 248, 46 kB, 1067 | 112, 22 kB, 1064 |
| `variable_text` | 140, 21 kB, 5653 | 75, 17 kB, 5641 |
| `jbig2` | 8327, 173 MB, 2782 | 2145, 32 MB, 2716 |
| `jpx` | 20506, 400 MB, 2252 | 353, 8.2 MB, 2225 |
| `cms` | 274, 2.4 MB, 193 | 105, 1.4 MB, 193 |
| `revocation` | 157, 7.5 MB, 605 | 92, 7.2 MB, 605 |
| `x509` | 538, 790 kB, 1043 | 388, 584 kB, 1042 |
| `object` | 10536 objects, 9.5 MB, 319 | 2117, 4.0 MB, 314 |
| `lexer` | 300, 27 MB, 194 | 179, 14 MB, 192 |
| `document` | 300, 27 MB, 2362 | 220, 18 MB, 2356 |
| `serialize` | 300, 27 MB, 2629 | 220, 18 MB, 2605 |
| `linearize` | 300, 27 MB, 4817 | 253, 22 MB, 4784 |
| `page` | 300, 27 MB, 18088 | 233, 19 MB, 17995 |

**`jbig2` is the one past the margin**, 66 edges (2.4%): an arithmetic decoder's contexts are its
data, and adding region dimensions bought five more edges for 500 more seeds. It is kept as it is.
The coarser structural shape the whole-document targets started with lost 4.6 to 4.8%; adding the
object kinds is what brought them inside.

## 3. What the population costs now

`fuzz/seeds.sh check` with the new recipes, behind the lock and `tools/bounded.sh --data 8`, one
target at a time, on 2026-10-06 — the thirteen ADR 1559 could not judge, and `object`:

| target | disk: seeds, `INITED cov` | fresh: seeds, `INITED cov` | verdict | wall |
|---|---|---|---|---|
| `cmap` | 3655, 368 | 107, 435 | **stale** | 336 s |
| `crypt` | 5127, 837 | 364, 1164 | **stale** | 169 s |
| `variable_text` | 6312, 6594 | 1094, 6734 | **stale** | 119 s |
| `object` | 5905, 462 | 60276 objects, 506 | **stale** | 1459 s |
| `jbig2`, `jpx`, `linearize` | none | 8701, 3363; 612, 2439; 13285, 6939 | **stale**: no corpus on disk | 354, 158, 114 s |
| `sfnt` | 1032, 383 | 6151, 358 | current | 227 s |
| `xmp` | 6632, 1584 | 25014, 1107 | current | 156 s |
| `ccitt` | 3740, 223 | 10920, 169 | current | 85 s |
| `revocation` | 771, 842 | 344, 665 | current | 951 s |
| `x509` | 1530, 1695 | 1095, 1184 | current | 456 s |
| `confined_wire` | 9338, 3688 | 44, 390 | current | 69 s |
| `cms` | 2668, 352 | 935, 248 | current | 2793 s |

The walls include the queue behind the lock. The first `cms` run stopped after 2259 s on an
`IndexError` in `seed_der.shape`'s walk, a long-form length running past its buffer; the walk is
guarded and was run over 193 800 mutated and cut CMS, certificate and revocation seeds without one. A current corpus's lead is what its campaigns found.
`cms` is the slow recipe because its `/TS` route inflates every stream of the 4735 documents that
name it (42 GB), a trade `seed_cms.py` states and this ADR leaves.

**The whole-document targets are not megabytes, and that is the price of the margin.** Over the
47 523 documents under 256 KiB the shapes that keep coverage are 10 370 documents and 982 MB for
`document` and `serialize`, 13 285 and 1.31 GB for `linearize`, 15 203 and 1.51 GB for `page`,
against 47 523 and 4.65 GB written whole; `lexer` is 2 768 and 96 MB and `object` 60 276 objects
and 119 MB. A shape coarse enough to be megabytes is the structural one section 2 measured losing
5%. A later round that wants these smaller reads the content streams, which is a parser in the
seeder; the census that printed these took 1765 s.

## 4. `check` builds without the sanitiser

AddressSanitizer reserves its shadow map before `main`, and `tools/bounded.sh`'s `RLIMIT_DATA`
refuses it, so a sanitised target under the bound prints no `INITED` and `check` said "failed".
`inited` now builds `cargo fuzz build -O -s none`, as `doc/verify.md`'s campaign does, so `check`
runs bounded and compares the figure the campaign will see. Figures are therefore not comparable
to ADR 1559's table digit for digit; each verdict compares two corpora in one binary.

## 5. `fuzz/artifacts`

`tools/state.sh fuzz` printed the same before and after: `page` 3 timeouts and 67 slow units,
`document` 2 slow, one crash each for `serialize` and `x509`, nothing for `jpeg_bands`. The two
crashes are not Q227's and each has its test (ADR 1559 section 4); both exit 0. The 70 `page`
units all exit 0 (median 2.2 s, at most 10.0 s without the sanitiser; at most 2.6 s in release
`interpret`) and are **six shapes**, by callgrind: ADR 1507's Type 3 cycle (3 units, 12.6 to 40.7
G instructions), nested forms and patterns reaching their bounds (18), image samples through the
`DeviceCMYK` press (29), substitute-font search (2), lexing and tiling (15), and three units of one
TrueType font with a format 2.0 `post` table of 5125 glyphs, 14.3 G each, 99% in `read_fonts`'s
`Post::glyph_name`, which `pdf-font`'s `post_glyph` calls per name and which walks the string list
from its start. That one is a quadratic no budget governs; it is priced in `doc/todo/49` and is
`pdf-font`'s to fix with a map built once per font.
