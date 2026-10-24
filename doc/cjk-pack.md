# The CJK pack — the second download, its layout, and how it is built

Owner answer A569: the four Adobe character collections' faces ship **as a second download beside
the base program**, so the base download keeps its size and its startup rules and a converted
document carries only the glyphs it shows. ADR 2302 is the decision and the route that embeds from
the pack; ADR 2303 is where the program looks for it and how it knows the files are the pack's;
`doc/questions/Q596` asks `doc/stack.md`'s question about the data dependency.

## What it is for

A composite font the file does not embed, whose descendant names `Adobe-Japan1`, `Adobe-GB1`,
`Adobe-CNS1`, `Adobe-Korea1` or `Adobe-KR`, under a predefined `CMap` other than the two identity
ones. ISO 32000-2 §9.7.2 makes a character collection's order what numbers its glyphs, so a CID of
the collection names the same glyph in any font of it, and a subset of the pack's program for the
collection can be written into the file in place of the program the producer did not embed. The
`quorra-transform archive` route names that as the loss `machine-face`: `as-if-printed` discards it,
`as-shown` stops (ADR 2162's reading). `Adobe-Identity` is not answered by the pack — its CIDs are
the producer's own glyph order — and what may answer it is the owner's open `doc/questions/Q570`.

The viewer does not read the pack; it draws such a font from a face of the machine, as before
(ADR 0270).

## Layout

One directory, named `cjk-pack`, unpacked **beside the `quorra-transform` executable**, or anywhere
else and named with `--cjk-pack <dir>`:

| file | what it is |
|---|---|
| `Adobe-Japan1.otf` | `Adobe-Japan1-7`'s program: 23 060 glyphs, glyph *N* is CID *N* |
| `Adobe-GB1.otf` | `Adobe-GB1-5`'s, 30 284 glyphs |
| `Adobe-CNS1.otf` | `Adobe-CNS1-7`'s, 19 179 glyphs |
| `Adobe-Korea1.otf` | `Adobe-Korea1-2`'s, 18 352 glyphs — every Korean `CMap` in Table 116 maps to this collection |
| `Adobe-KR.otf` | `Adobe-KR-9`'s, 22 897 glyphs — the collection §9.7.5.2 names |
| `LICENSE.txt` | the SIL Open Font License 1.1 with the family's copyright notice, verbatim |
| `SHA256SUMS` | each file's SHA-256, in `sha256sum`'s format |

Each program is an OpenType font with a CID-keyed `CFF ` table whose `ROS` is the collection's and
whose charset gives glyph *N* the CID *N*, every charstring desubroutinized so that its width is its
own first operand — the converter restates a glyph's advance to the file's `/W`, and a width reached
only through a subroutine is a site that rewrite does not follow. The five come to about 27 MB. A CID the builder found no published mapping for is a
glyph with no outline; the converter refuses a page that shows one unless the collection's own
`Adobe-*-UCS2` table makes the CID a space.

**The program never fetches the pack.** A person downloads it and unpacks it; without it, every
such font is refused with a sentence saying the pack is not installed and where it goes.

## How it is built

```
python3 -I -B tools/cjk-pack.py --source <dir> --cmaps data/cmaps --out <empty dir>
```

`<dir>` holds Source Han Sans 2.005R's region-specific subset programs —
`SourceHanSansJP-Regular.otf`, `SourceHanSansCN-Regular.otf`, `SourceHanSansTW-Regular.otf`,
`SourceHanSansKR-Regular.otf`, from the `release` branch's `SubsetOTF/<region>/` of
`https://github.com/adobe-fonts/source-han-sans` — with the family's `LICENSE.txt` and
`SourceHanSans_JP_sequences.txt` from the same repository. The script checks each program against
the SHA-256 it names before reading it, and refuses an output directory that is not empty.

A CID is filled only through a mapping Adobe publishes for its collection: the collection's
`Uni*-UTF32-H` `CMap` (the program's `cmap` glyph for that Unicode value), its `-V` twin through the
program's `vert` substitution, `Adobe-Japan1`'s `UniJIS-UCS2-HW-H`/`-HW-V` through `hwid`, and the
Ideographic Variation Database's `Adobe-Japan1` sequences through the program's format 14 `cmap`.
The `CMap`s are the ones the tree already carries in `data/cmaps/`.

Each program is a Modified Version under the OFL, so none keeps the Reserved Font Name "Source": it
is `QuorraCJK-<Ordering>` in its `name` table, its CFF Name INDEX and its Font DICTs, and keeps the
family's copyright notice. Every subset the converter writes from it carries `LICENSE.txt` verbatim
in its program stream's `/Metadata` packet under `xmpRights:UsageTerms` (A550's pattern).

**The build is reproducible**: two builds from the same inputs under fontTools 4.66.1 wrote the same
bytes, and
`crates/pdf-transform/src/archive/fonts/pack.rs` carries those bytes' digests. A pack built from
anything else is refused by name rather than embedded.
