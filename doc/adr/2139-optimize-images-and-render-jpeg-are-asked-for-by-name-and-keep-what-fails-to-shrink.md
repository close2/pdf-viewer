# 2139 — `optimize --images` and `render --format jpeg` are asked for by name, and keep what fails to shrink

Session 1651, the archive slot of batch one hundred and one. Status: **accepted**, **built** in
`pdf-transform`. It **replaces** ADR 0842 section 5's refusal. Context: `doc/questions/A01` ("be
very careful to only reencode when explicitly requested"), ADR 2138 (the encoder), ADR 0842
section 3 (the keep-the-original rule), RFC 0002 sections 6.4 and 6.5. Code:
`crates/pdf-transform/src/optimize_images.rs`, `optimize.rs`'s `copy_closure_with`, `render.rs`'s
`ImageFormat::Jpeg`, `images.rs`'s `ImageFile::JpegEncoded`, the command line's `image_format` and
`image_recompression`.

## 1. Asked for, never a default

The owner's condition is the design: no profile, no default and no fallback reaches the encoder.
`optimize` re-encodes only under `--images quality=<n>`; `render` and `images` write JPEG only under
`--format jpeg` (or `jpg`), with `--quality` refused unless that word was given; `images --native`
still writes a DCT image's own bytes. `tests/optimize_images.rs` holds the first half: the lossless
default over the fixture writes no `DCTDecode` and counts nothing.

## 2. Which images, and why each refusal is a clause's

`optimize_images.rs`'s module documentation is the list, and each line is a clause: eight bits a
component and one or three of them in a colour-valued space (§7.4.8: "[e]ach component value shall
occupy a byte"), never `Indexed` (§8.6.6.3's samples are indices), never a stencil (§8.9.6.2),
never a colour key (§8.9.6.4 masks exact samples), never an image already under a codec (re-encoding
a lossy one loses twice), and ADR 0842 section 3's conditions for every stream — no `/F`, no
`/Crypt`, no failed decryption, no damaged decode, and no `/Filter` or `/DecodeParms` naming another
object, which the re-encoded stream states no longer. Every other entry crosses unchanged.

## 3. Smaller, or the producer's bytes

qpdf's `--optimize-images` rule, ADR 0842's rule for every stream: the JPEG replaces the stream only
where it is smaller than the producer's stored bytes. And where the lossless pass would deflate in
front of the codec (`[/FlateDecode /DCTDecode]`, smaller), the replacement is written that way at
once — found by the idempotence test, which failed until it was: a second run's lossless pass
wrapped the first run's JPEGs. With it, optimising this verb's lossy output writes the same file.

**The numbering is the lossless run's.** The walk asks the pass before it copies each object; an
image it replaces takes `Assembly::replace`'s slot at the place a copy would have had, and its
references are walked as a copy's are, so `--images` changes the images' bytes and nothing else's
position. The slots are filled once the walk has numbered everything their dictionaries name.

## 4. `downsample=` is refused by name

RFC 0002 section 6.5's other knob resamples to a dpi "at their placed size", and an image's placed
size is every `Do` that draws it under each one's CTM — a walk of every content stream, which this
verb otherwise never interprets. It is a usage refusal naming that, and `doc/todo/57` carries it.

## 5. Two choices, stated as choices

- **`render`'s default quality is 90** (`render::JPEG_QUALITY`). §7.4.8 leaves the loss to the
  encoder; a rendered page is mostly text and line art, whose edges ring first.
- **A mask beside an `images --format jpeg` file is a PNG**: the opacity is carried losslessly
  whatever the picture beside it was asked to lose.

## 6. What a JPEG the encoder refuses becomes

In `render`, a page whose raster is past the encoder's 2^26 samples is a refusal of that page (exit
4, the other pages written), not a sink failure. In `optimize`, an image the encoder refuses keeps its
bytes and the report says so in a warning naming the object.
