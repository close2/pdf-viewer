# ADR 0019 — An image written into the content stream

Status: accepted, 2026-07-29.

## Context

ISO 32000-2 §8.9.7 lets a small image be written *inside* a content stream rather than as an
image `XObject`: `BI`, a dictionary of bare key–value pairs, `ID`, the samples, `EI`. Until
this session the interpreter recognised the three operators, skipped to `EI` and reported
`Image { name: "<inline>" }` — 22 corpus documents, the largest named image gap, and ten of
them were the tenth session's own doing, because a Type 3 font whose glyphs are inline image
masks began running its glyph descriptions and drawing nothing (ADR 0018).

The clause is short and three of its rules are not shared with any other image.

## The decision

`crate::inline_image::scan` turns a `BI` … `EI` sequence into **the `Stream` the same image
would have been as an image `XObject`** — the full key names, the full filter and colour space
names, the data — and `draw_image` takes it from there. One decoder, one colour conversion,
one `Command::Image`; nothing downstream knows an inline image from any other, which is the
rule trap 6 in `doc/HANDOVER.md` exists to protect.

That decision is what makes the rest of the clause small:

- **Table 91's abbreviated keys and Table 92's abbreviated names** are expanded once, at the
  boundary. `/BPC` becomes `/BitsPerComponent`, `/Fl` becomes `/FlateDecode`, `/RGB` becomes
  `/DeviceRGB`. The one place the same letter means two things is `I`: `Interpolate` as a key
  and `Indexed` as a colour space family, which is why key expansion and value expansion are
  separate tables rather than one.
- **Entries the clause does not list are dropped** — "Entries other than those listed shall be
  ignored". `/SMask` is not on Table 91, so an inline image has no soft mask however it spells
  one, and the dictionary that comes out is one an image `XObject` could have had.
- **`/CS` may name a resource.** A name that is not a device space is a key into the page's
  `/ColorSpace` subdictionary (§7.8.3), and NOTE 3 says the six device names never are. An
  image `XObject` never needs this lookup, which is why it is the scanner's job rather than
  the decoder's.

## Where the data ends, which is the whole difficulty

An inline image's samples have no `/Length` unless the file is PDF 2.0, and the bytes are not
PDF syntax, so a reader that mislocates the end does not fail: it tokenises image data as
operators. Three answers, tried in this order, each checked against the `EI` it predicts:

1. **`/L` (or `/Length`)**, which the clause requires of a PDF 2.0 file and defines exactly —
   "the length of the data between the ID and EI operators excluding the white-space
   delimiting those operators". Checked rather than believed: a wrong length would otherwise
   swallow the rest of the page.
2. **Arithmetic, where there is no filter.** §8.9.3 fixes the layout of samples — row order,
   each row padded to a byte boundary — so `/W`, `/H`, `/BPC` and the colour space's component
   count give the byte count with nothing left to guess. This is the answer for a Type 3
   glyph bitmap, which is unfiltered and one bit deep.
3. **A search for a token-delimited `EI`**, which is the only guess in the module and is
   reached only for filtered data in a file with no `/L`.

Two tolerances are deliberate and both were found by corpus documents rather than reasoned
into existence:

- **`ID` followed by CRLF.** The clause says "a single white-space character", and
  `bug1065245.pdf` writes two. §7.2.3 defines an end-of-line marker as CR, LF, **or CR
  followed immediately by LF** — one marker — which is the same rule §7.3.8.1 applies after
  the `stream` keyword. Reading the LF as the first byte of a JPEG starts the codestream one
  byte late, and the image is refused with a confident-sounding error.
- **`EI` with no white space before it.** The clause's own sentence puts white space there,
  and requiring it is what keeps an `EI` *inside* compressed data from ending an image early.
  So one walk collects both: a delimited terminator ends it immediately, and an undelimited one
  is remembered and used only if nothing better turns up — which recovers the thirteen inline
  images in `issue19532.pdf` whose `EI` sits hard against the last data byte. One walk rather
  than two searches, because the input that provokes the fallback is a stream of unterminated
  `BI` operators and that is a shape a hostile file can write cheaply.

## What it uncovered

The feature was the smaller half of the session. Drawing an image that had never been drawn
put four more things on the screen, and each was a gap *inside* something already implemented
— the shape this project keeps finding and the reason trap 1 says to look at the page.

**`/Interpolate` was read nowhere (§8.9.5.3).** `issue11124.pdf` draws sixteen samples across
a hundred pixels; three references draw sixteen flat squares and we drew a blur, because both
backends filtered every image bilinearly. The clause makes interpolation a hint about
*magnification* with a default of false, so `Image::is_smoothed` now answers one question for
both backends: filter a reduced image, and draw a magnified one sample by sample unless the
document asked otherwise. The reduction half is a documented choice — the clause says nothing
about it, and nearest-neighbour there would drop samples outright.

**`Indexed`, `Separation`, `DeviceN` and `Lab` images did not unpack.** They were refused with
`colour space Indexed is not supported` while the same spaces had worked for fills since the
sixth session. Routing them through `crate::colour` — the one place a colour becomes RGB —
took another ten documents off the corpus's incomplete list, and needed §8.9.5.2 Table 88's
one exception: every space's default `/Decode` maps a sample onto 0.0 to 1.0 **except**
`Indexed`, whose `[0 2^n - 1]` passes an index through unchanged. An index divided by 255 is
index 0 for every sample but the last.

**An `Indexed` table's bytes were not scaled into the base space's range (§8.6.6.3).** "Each
byte shall be an unsigned integer in the range 0 to 255 that shall be scaled to the range of
the corresponding colour component in the base colour space". Dividing by 255 is that scaling
only where the components run 0 to 1, which is every family but `Lab`. `issue2761.pdf` indexes
into a `Lab` base and drew a **black square** where four renderers draw a pale grey gradient:
its lightest entry is L = 253, which is 0.99 as a fraction and 99 out of 100 as a lightness.
The defect predates this session by five and was invisible because no `Indexed` image drew.

**An annotation appearance with no `/Resources` was run against an empty dictionary
(§7.8.3).** An appearance stream is a form `XObject` (§12.5.5), and the clause's last bullet
says a form that omits `/Resources` inherits the page's. Forms got that; appearances did not,
so every font and image such an appearance named was lost. Found by reading the clause the
inline-image scanner cites for its `/ColorSpace` lookup.

## Consequences

- **13 documents left the corpus's incomplete list and 9 kept an inline image on it** — now
  naming `CCITTFaxDecode` or a bit depth instead of the bare word `<inline>`, which is the
  difference between a gap and a mystery. `Indexed` and friends took another 10.
- **Three documents came back**, and they are the honest kind of rise: a soft mask whose
  sample grid is not its image's is expressly permitted by §11.6.5.2 Table 143 and is not
  applied here, and all three had been drawing an unmasked image in silence. `issue16263.pdf`
  puts black bars across its text.
- **42 more pages entered the oracle's comparison**, four of them contradicted: two are the
  one-pixel page-rounding difference that already has a group, one is a `CalRGB` alternate
  space we convert and two references do not, and one is an image half a pixel tall that we
  antialias and they snap to a whole row. Each is named with its argument in `oracle.rs`.
- **A performance defect the feature walked into.** `apply_soft_mask` decoded a mask and
  *then* compared its dimensions, so `issue16263.pdf`'s 34862×4332 mask over a 2×2 image spent
  19 seconds and 600 MB producing a raster discarded on the next line. Asking the dictionary
  first costs nothing. Separately, a one-component space's colours are now tabulated once
  instead of per sample: 1.03 s to 330 ms over the sixteen corpus documents that draw one, and
  620 ms to 42 ms on `issue9940.pdf`, whose `Indexed` table sits over a `DeviceN` with a
  PostScript tint transform.

## What is still owed

`/Interpolate` is honoured as a magnification hint and *ignored* for reduction, which is where
`firefox_logo.pdf` still misses the oracle's bound. A general `/Decode` array — any linear map
rather than the fully-inverted case — is still not applied and still silent; §8.9.5.2's ledger
row records it. And an inline image naming `JPXDecode`, `JBIG2Decode` or `Crypt` is decoded
rather than refused, which the clause forbids to a *writer*; refusing it would fail a
malformed file that currently renders.
