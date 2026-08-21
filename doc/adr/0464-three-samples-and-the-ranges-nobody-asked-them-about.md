# ADR 0464 — Three samples, and the ranges nobody asked them about

Status: accepted, 2026-08-21. Session 631. Takes `doc/todo/03` §21's named successor — the next
chunk of the SafeDocs crawl — and fixes the three defects its ten thousand documents produced.
Amends §7.4.2's, §7.4.3's, §7.4.9's and §8.9.7's ledger rows, and adds a row to
`doc/errata-read.md`.

## The chunk, and why these archives

§21 leaves "33 944 crawled documents unranked … in archive-sized pieces". This round took **ten
whole archives — `0792`, `1038`, `1776`, `2145`, `2883`, `3621`, `4359`, `5097`, `5835` and
`6573`, 10 000 documents** — none of the thirty-two archives sessions 603, 613, 615, 619 and 625
ranked. *Which* archives is immaterial and that is ADR 0261's finding: the crawl is sorted by
SHA-256 and cut into equal pieces, so an archive is a hash bucket and any set of them is an
unbiased sample.

The instrument is 603's, unchanged and reused rather than rewritten: page one at 72 dpi from this
tree and from `pdftoppm`, `mutool` and `gs`, every invocation copied from
`tools/pdfref/src/reference.rs` and explicit about the page box (trap 3), ranked by our ink minus
the lightest **live** reference's — a panel of zero ink is not a page — with each panel's raster
size beside the number. **15 minutes for the ten thousand** at sixteen workers.

Two checks before anything was read, both of them somebody else's lesson taken rather than
re-learnt:

- **Both binaries built** (619): `cargo build --release -p pdf-model --example render_at` *and*
  `cargo build --release -p pdf-sandbox --bins`, with `target/release/examples/` confirmed to hold
  no stale `pdf-sandbox-worker` of its own (624, ADR 0458).
- **§20's check run** (623): `cargo test --profile gates -p pdf-model --test fixed_documents --
  --ignored` — **25 checked, 0 absent, 25 rows, green** — and the instrument itself reproduced
  ADR 0459's three recorded documents to the thousandth (`0669424.pdf` +0.181, `4113230.pdf`
  −0.102 against a recorded −0.103, `0100223.pdf` −0.158) before a new row was looked at.

## What the two ends said

**The negative head is deep and the positive head is not flat**, which is the first chunk where
the row worth taking sat on the *positive* side. Three defects of this tree, and all three are one
question asked in three places: **what range does a sample run over?** Not one of them is about
decoding; every one is about the arithmetic between a decoded byte and a colour or an extent.

The rest of the head is known shapes, named here so the next round does not re-derive them:
`5835546.pdf` −13.310 is `MAX_OPERATIONS`, which is `doc/todo/49`'s and is priced there;
`2883767.pdf` −7.159 reports §11.4.4's non-isolated group, which is `doc/todo/23`'s; and below
+20 the positive head is 613's finding — `poppler` alone drawing almost nothing while the other
two and this tree agree — read out of `doc/traps/oracle-and-references.md`. `5097568.pdf`
+26.596 and `4359131.pdf` +20.057 are that note's other half with a different reference light:
`mutool` at 61.783 against 92.223 / 77.597 and at 112.353 against 133.126 / 132.219, with ours
within a level of `poppler` and `gs` on both.

**61 rows of the 10 000 produce no number**, the same three shapes 613, 615, 619 and 625 opened by
hand.

## 1. A photograph's fourth channel, taken for opacity by a space the clause says to ignore

`0792405.pdf` at **−8.329** — ours 13.316 against 22.757 / 21.644 / 22.010 — is a magazine page
whose two `/JPXDecode` photographs are both missing and both reported:

```
Im1: malformed image: the colour space takes 4 components but the codestream has 3
```

**The codestream has four.** `opj_dump` says `numcomps=4`, and so does its SIZ marker read by
hand. What it does *not* have is any JP2 box at all: it opens `FF 4F FF 51`, a bare codestream,
so there is no colour specification box and no channel-definition box either.
`hayro-jpeg2000`'s `j2c::parse` synthesises a colour space for that case — greyscale below three
components and sRGB at or above — and `resolve_alpha_and_color_space` then finds four channels
beside a three-channel space and concludes, in its own comment, "[a]ssume that we have an alpha
channel in this case". That is a defensible guess about a file that says nothing.

§7.4.9 says the file is not saying nothing, because the *dictionary* is:

> If present, it shall determine how the image samples are interpreted, and the colour space
> specifications in the JPEG 2000 data shall be ignored. The number of ordinary colour channels
> in the JPEG 2000 data shall match the number of components in the colour space

Which channel is an opacity channel is read off those same specifications — the channel-definition
box sits beside the colour specification box in the same JP2 header — so a classification derived
from a synthesised colour space is a colour space specification in the JPEG 2000 data, and the
sentence sets it aside. Table 87 says the same thing from the other side: with `/SMaskInData` 0 or
absent, encoded soft-mask information is ignored, so reading the fourth channel as colour loses
nothing the file asked for.

`image::decode_jpx` now takes every channel as an ordinary colour channel **exactly when** the
dictionary states the space, `/SMaskInData` is 0 or absent, and the declared space accounts for
every channel that arrived. The three conditions are the point rather than the fix: a non-zero
`/SMaskInData` is the file *stating* an opacity channel and is believed — §7.4.9: "[i]f
SMaskInData is non-zero, there shall be only one opacity channel in the JPEG 2000 data and it
shall apply to all colour channels" — and a space whose ordinary channels already match is left
alone. Anything wider would be "make the counts agree", which is not a reading of anything.

−8.329 → **+0.576**, nothing reported.

`tests/jpx_channels.rs` pins it with a generated eight-by-eight four-component bare codestream —
`opj_compress` over four constant planes, its `COM` marker stripped so no encoder version is baked
in — and the two negative twins, `/SMaskInData 1` and `/DeviceRGB`, which must keep the refusal and
the opacity channel respectively.

## 2. An extent a filter states and this reader was searching for

`5097148.pdf` at **−43.503** is the deepest row of the ten thousand: ours **0.092** — a blank
sheet — against 44.214 / 43.596 / 44.042. It reports an inline image whose samples "stop at 75393
bytes where 2951x178 at 8 bits and 3 component(s) needs 1575834", and then **several hundred**
`Operator` reports whose names are runs of base-85: `!oqls inside an array, which §7.3.6 admits
only objects into`.

The image is `/W 2951 /H 178 /BPC 8 /CS /RGB /D […] /F [/A85 /Fl]` with no `/L`, so
`inline_image`'s answers 1 and 2 do not apply and answer 3 runs — the forward search the module's
own comment calls "the one guess". The first `EI` standing as its own token is **69 598 bytes**
into 1.29 MB of base-85, and everything past it went to the lexer as a program.

This is the extent ADR 0459 named and left, and the clause answers it more sharply than that note
expected. §8.9.7:

> The bytes between the ID operator and a white-space token, but before the EI operator shall be
> treated the same as a stream object's data ( see 7.3.8, "Stream objects"), even though they do
> not follow the standard stream syntax.

A stream's data ends where its own filter says it does, and **two of the filters Table 92 admits
say so in the data**: §7.4.2's "A GREATER-THAN SIGN (3Eh) indicates EOD (End Of Data)" and
§7.4.3's two-character (7Eh)(3Eh), over an alphabet of `!` through `u` and `z` in which neither
byte can otherwise occur. Which filter is asked is Table 5's: "Multiple filters shall be specified
in the order in which they are to be applied", so the *first* entry of the array is the one whose
input the bytes after `ID` are. And the clause's own EXAMPLE is exactly this arrangement — a
`/F [/A85 /LZW]` image whose data ends `R.s(4KE3&d&7hb*7[%Ct2HCqC~> EI`.

So `data_extent` has a third derived answer between the arithmetic and the search, checked against
the `EI` it predicts like the other two, and — the half ADR 0454 paid for — a marker absent from a
*window* asks for more bytes rather than letting the search run.

−43.503 → **−0.323**, nothing reported, 1 command → 328.

**`spec-errata emit` over the clause family is what made this a rule rather than an inference**,
and it is the reason `doc/errata-read.md`'s standing instruction exists. Errata Collection 3's
Issue #293 adds a whole sentence to §7.4.3: "If the ASCII85Decode filter encounters the character ~
in its input, the next character shall be > and the filter will reach EOD. Any other characters
shall cause an error." — with a NOTE crediting the PostScript Language Reference Manual's clause
3.13.3. `spec-errata check` had never named it and could not: it compares the *tree's quotations*
against struck passages, and this is a pure addition over text nobody had quoted. That is ADR
0187's §5.1.3 lesson one clause family along, and the row is now in `doc/errata-read.md`.

`tests/inline_images.rs` pins the base-85 arm on data that spells a white-space-delimited `EI` of
its own, the hex arm on the same shape, and the window twin — a marker outside the held bytes is
`Truncated`, never searched.

## 3. A lightness of one where a hundred is white

`4359750.pdf` at **+32.097** — ours 72.337 against 41.626 / 40.240 / 42.013, three references
agreeing within 1.8 — is a schoolbook page whose photograph this tree draws as a **solid black
rectangle**, silently. Nothing is reported and 414 commands are interpreted; the page is otherwise
perfect, which is why only a positive ink gap could find it.

The image is `/DCTDecode`, 543×372, three components, and its `/ColorSpace` is

```
[ /Lab << /BlackPoint [0 0 0] /Range [-128 127 -128 127] /WhitePoint [.964203 1 .824905] >> ]
```

`convert_three` — the `DCTDecode` route's conversion from the dictionary's space into device RGB —
divided every eight-bit channel by 255 and handed the three quotients to the space. §8.6.5.4 makes
a `Lab` space's first component a **percentage** and its other two the space's own `/Range`, so a
lightness of at most 1 arrives where 100 is white and every colour in the photograph collapses onto
black.

§8.9.5.2 states the map and Table 88 states its ends:

> Samples with a value of 0 shall be mapped to D min … those with intermediate values shall be
> mapped linearly between D min and D max

with the default pair taken per *space* where the dictionary states no `/Decode` — `[0 1]` for the
device families, `[0 2^n − 1]` for `Indexed`, and `[0 100 amin amax bmin bmax]` for `Lab`. The
unfiltered route has always unpacked through exactly that table (`Decode::read`); this route had a
constant instead.

**The `Indexed` half of the same hole was found one arm along and fixed in the
six-hundred-and-thirteenth** (ADR 0448, `6327194.pdf`, a solid black page for the same reason with
a palette instead of a percentage) — and the fix that landed then was a `scale` of 1.0 in the
one-component arm, which is Table 88's `Indexed` default written as a constant rather than read.
The three- and four-component arms were not touched and had no witness until now. So the shape to
recognise is not "Lab": it is a **sample-to-component map written as a division** in a route that
already had the map built beside it.

`convert_channels` now builds `Decode::read` for the resolved space and the three arms index it.
Two consequences beyond the defect, both improvements and both stated rather than smuggled: the
one-component arm's `Indexed` special case disappears into the table, and an explicit `/Decode`
array reaches this route in the space's *own* component values instead of through
`apply_decode_to_channels`'s eight-bit channel remap — which is still what runs for `DeviceGray`,
`DeviceRGB` and a stencil, where a channel and a component are the same thing.

+32.097 → **+0.307**, and the photograph is the photograph.

**The diagnosis was made outside this tree before the fix was written** (trap 8): `mutool extract`
pulls the codestream out, `PIL` decodes it, and Table 88's default decode applied in twenty lines
of Python reproduces the reference renderers' photograph. That is what turned "our Lab conversion
is wrong somewhere" into a map with two ends.

## What moved, measured — and the instrument that measured it

**The reach of a change to this tree is exactly the set of documents whose *own* panel moves.** A
reference renderer's panel cannot depend on our build, and ADR 0459 measured that one *does* differ
between two runs with nothing changed — nine rows of its own thirty-nine were that. So this round
measured the reach over our column alone, before and after, across all **42 archives / 42 000
documents** any chunk round has ranked: same page, same 72 dpi, same ink, no reference process
spawned. It removes that noise by construction and costs a quarter of the wall clock, which is what
made a whole-population before-and-after affordable on a machine three other rounds were building
on.

**Ten rows of the 42 000 move. Six are the three fixes and four are the instrument.**

| document | before → after | which fix |
|---|---|---|
| `5097/5097148.pdf` | 0.0924 → 43.2732 | the extent |
| `4359/4359750.pdf` | 72.3370 → 40.5470 | the decode range |
| `4482/4482885.pdf` | 67.7896 → 55.6610 | the decode range |
| `0792/0792405.pdf` | 13.3155 → 22.2199 | the channels |
| `0423/0423614.pdf` | 43.1913 → 42.7474 | the extent |
| `7311/7311510.pdf` | 26.5206 → 26.5402 | the decode range |

**Three of the six are in archives an earlier chunk took**, which is the sixth round running that a
fix has reached back: `4482` and `0423` are 615's, `7311` is 625's. Put in front of the three
references afterwards, every one of them moves *toward* agreement — `4482885.pdf` **+11.288 →
−0.840** against 56.501 / 56.943 / 56.634, `0423614.pdf` +0.487 → +0.043, `7311510.pdf` −0.278 →
−0.259. `4482885.pdf` is a second `/Lab` `DCTDecode` page and was a whole page too heavy;
`0423614.pdf` is a 2448×3024 scan under `/A85`; `7311510.pdf` is the decode map's *other* half —
`/Indexed` images stating `/Decode [0 255]`, which this route now honours in the space's own
component values instead of as an eight-bit channel remap.

The other four rows produce no number in the before pass and one in the after: `4482/4482567.pdf`,
`5589/5589666.pdf`, `0546/0546114.pdf` and `0546/0546365.pdf`. Re-run on a quiet machine, **both
binaries give byte-identical numbers on all four**, so they are a render that lost its ninety-second
budget while three other rounds were compiling — the same shape ADR 0459 recorded from the other
column, and 626's lesson about load arriving as a false regression.

**And the first fix's population is one document, measured with an instrument that is not this
tree's** (trap 8): a walk over all **65 944** crawled documents that reads each `/JPXDecode` stream's
own SIZ marker and its dictionary with a regular expression finds **one** document carrying a bare
codestream whose component count is neither 1 nor 3 — `0792405.pdf`, with eight such images, every
one of them under a stated `/ColorSpace`. That is a narrow population and the row says so; what
makes the fix worth having is that the clause is not narrow, and a JP2 file that states four
channels and a `/DeviceCMYK` dictionary would have met the same arithmetic.

## What the head still holds

- **`7926872.pdf` −41.731 is still open, and answer 3 is still the reason.** Its inline image is
  `/F /FlateDecode`, which states no marker in its data, so this round's third answer does not
  reach it; what it needs is what ADR 0459 named — `pdf_syntax::Pump` counting consumed input on
  its Flate engine and exposing the count. The two arms are now visibly different questions rather
  than one, which is worth the distinction: a *textual* end-of-data is derivable without decoding,
  a structural one is not.
- **Three silent rows this round diagnosed no further than their numbers.** `2883994.pdf` −10.134
  (5236 commands), `1776488.pdf` −5.965 (198 commands, ours 199.525 against 205.490 / 210.506 /
  210.780) and `5835193.pdf` −4.976 (120 513 commands). None is called a family here: a structure
  count is evidence about where to look and never about who is right (trap 9).
- **619's four, 615's two and 625's five and one remain**, each named in `doc/todo/03`.
