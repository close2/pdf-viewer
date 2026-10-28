# veraPDF reads a JPEG 2000 file's opacity channel as a colour channel — an upstream issue, drafted for the owner to file

Status: **a draft, not filed.** Written by session 1843 under `doc/questions/A691` ("the clause's
text governs"; "shouldn't we open an issue with verapdf?"). Nothing here has been sent; the owner
files it, or says not to. It is indexed from `doc/pdfa-blocking-issues.md` section 9.

## Where it goes

veraPDF's issue tracker, against the library that holds `org.verapdf.external.JPEG2000`, class
method `parseHeader` (the class is in the `veraPDF-library` or `veraPDF-parser` repository; whichever
carries the class takes the issue). Measured on veraPDF 1.30.3, the build in `/usr/bin/verapdf` on
the machine the rounds run on (`verapdf --version` prints `Built: Wed Oct 07 13:52:00 CEST 2026`).

## Title for the tracker

`JPEG2000 header parsing counts the opacity channel as a colour channel (ISO 19005-2 6.2.8.3, ISO
19005-4 6.2.7.3)`

## The report, as it can be pasted

veraPDF fails two valid JPEG 2000 images at the first and the fifth test of ISO 19005-2 clause
6.2.8.3 (ISO 19005-4 clause 6.2.7.3 for part 4). Both clauses state their requirement of the
**colour channels**: that there are 1, 3 or 4 of them, and that they share one bit depth. The images
below have the colour channels the clauses ask for, and an additional channel that ISO/IEC 15444-1
Annex I types as opacity in its Channel Definition box. veraPDF applies both tests to every channel
of the file.

### Reproducer 1: one grey channel and one opacity channel, both 22 bits

File `verapdf-jpeg2000/grey22-opacity22.2b.pdf` (PDF/A-2b) and `grey22-opacity22.4.pdf` (PDF/A-4).

```
$ verapdf --flavour 2b --format xml grey22-opacity22.2b.pdf
rule 6.2.8.3, test 1
  JPEG2000 image has 2 colour channels that is neither 1, 3 or 4
$ verapdf --flavour 4 --format xml grey22-opacity22.4.pdf
rule 6.2.7.3, test 1: the same description and message
```

The JP2 file's boxes are `ihdr` with NC = 2 and BPC = 0x15 (22 bits), `colr` with enumerated colour
space 17 (greyscale), and `cdef` with two entries: channel 0 of type 0 (colour) associated with
colour 1, channel 1 of type 1 (opacity) associated with the whole image. That is one colour channel.

### Reproducer 2: three 20-bit colour channels and one 18-bit opacity channel

File `verapdf-jpeg2000/rgb20-opacity18.2b.pdf` and `rgb20-opacity18.4.pdf`.

```
$ verapdf --flavour 2b --format xml rgb20-opacity18.2b.pdf
rule 6.2.8.3, test 5
  JPEG2000 image has different bit-depth parameters in 'bpcc' box, or bit depth is out of range
  ('bpcc' box present = true, bit depth = 0)
$ verapdf --flavour 4 --format xml rgb20-opacity18.4.pdf
rule 6.2.7.3, test 5: the same description and message
```

The boxes are `ihdr` with NC = 4 and BPC = 0xFF (the value that says the depths are in a `bpcc`
box), `bpcc` with 20, 20, 20 and 18 bits, `colr` with enumerated colour space 16 (sRGB) and `cdef`
with three colour entries and one opacity entry. The three colour channels have one depth.

### What the decompiled class does

Read from `cli-1.30.3.jar` by `javap -c` on `org.verapdf.external.JPEG2000`; the offsets are
bytecode offsets of `parseHeader`.

1. **The Image Header box's component count is taken as the colour-channel count.** At `ihdr`
   (offsets 221 to 269) the 2-byte NC field (ISO/IEC 15444-1 I.5.3.1: the number of components in the
   codestream, all channels) is read and passed to `Builder.nrColorChannels`. NC = 4 for red, green,
   blue and opacity passes test 1 only because 4 is also an admitted count (CMYK); NC = 2 (grey and
   opacity) and NC = 5 (CMYK and opacity) fail it.
2. **The `bpcc` box is skipped whole and its presence is the finding.** At `bpcc` (offsets 331 to
   356) the box is passed over with `skipBytes` and `bpccBoxPresent` is set, and test 5 checks that
   the flag is false. ISO/IEC 15444-1 I.5.3.2 makes the box the place where a file states component
   depths that differ, which includes an opacity channel of another depth; whether the *colour*
   channels differ is the question the clauses ask, and the box has to be read, with the
   `cdef` box (finding 3) deciding which entries are colour, to answer it.
3. **The Channel Definition box is never read.** `parseHeader` handles `ihdr`, `bpcc` and `colr`
   inside `jp2h` and no other box. `cdef` (I.5.3.6) is the box that types each channel as colour,
   opacity or premultiplied opacity and associates it with a colour; without it the count and the
   depth of the colour channels cannot be told from those of the file.
4. **The sign bit of BPC is not masked.** At offsets 302 to 317 the `ihdr` BPC byte is widened as a
   signed Java `byte` and incremented, without `& 0x7F`. I.5.3.1 puts the depth in the low seven
   bits and the signedness in bit 7, so a signed component of any depth yields a negative bit depth
   and fails the range test. No reproducer is attached for this one: the writer that made the two
   above writes unsigned samples only.

### What each clause says (cited, not quoted)

- ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3: the number of colour channels, the
  range of the bit depth, and the equality of the depth across the **colour** channels.
- ISO/IEC 15444-1 I.5.3.1 (image header), I.5.3.2 (bits per component), I.5.3.6 (channel
  definition: `Typ` 0 colour, 1 opacity, 2 premultiplied opacity).

### Why a file like this is valid

ISO 32000-2 section 7.4.9 and the `SMaskInData` entry of Table 87 let a `JPXDecode` image carry an
opacity channel inside the file (the entry says whether the processor uses it), and the clauses above do not mention that
channel. An image with a colour channel count the clauses admit and one common colour depth is
inside both clauses whatever its opacity channel does.

## How the reproducers are made

They are the outputs of this repository's PDF/A converter for a source whose JPEG 2000 data is deeper
than sixteen bits. The converter writes such an image back as a JP2 file with `pdf_jpx_write` (ADR
2484) and, since ADR 2521, writes these two shapes too.

```sh
# the fixtures: crates/pdf-transform/tests/archive/deep_jpx.rs
#   a_deep_jpx_off_the_baseline(&[22], Some(22))        grey 22 bits, opacity 22 bits
#   a_deep_jpx_off_the_baseline(&[20, 20, 20], Some(18)) colour 20 bits, opacity 18 bits
# each placed as the one image of a PDF/A-2b page that draws it, with /SMaskInData 1
# (`a_page_with_the_image` there, plus the content stream q 80 0 0 80 10 10 cm /Im Do Q)
quorra-transform archive grey22-opacity22.pdf --to 2b --profile keep-everything -o grey22-opacity22.2b.pdf
quorra-transform archive grey22-opacity22.pdf --to 4  --profile keep-everything -o grey22-opacity22.4.pdf
```

The source files are `verapdf-jpeg2000/grey22-opacity22.pdf` and `rgb20-opacity18.pdf`; their JPEG
2000 data names an enumerated colour space the baseline of T.801 M.9.2 does not hold, so they fail
the baseline clause (which is why a converter touches them) and are not themselves reproducers.
The converted files are: veraPDF's only failure on each of them is the one above, and this
repository's own checker (`pdf-archive`), which reads the colour channels as the clauses state,
passes both at the target they were written for.

## What this repository does meanwhile

`doc/questions/A691` rules that the clause's text governs: the two refusals the converter kept for
these shapes were removed (ADR 2521), and a depositor whose archive runs veraPDF 1.30.3 sees the two
failures above on a deep image that carries an opacity channel of another depth than its colour
channels, or a grey image with an opacity channel. Nothing in the corpora has either shape.
