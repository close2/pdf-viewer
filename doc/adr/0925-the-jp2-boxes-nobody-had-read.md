# 0925 — The JP2 boxes nobody had read, and where the reader for them goes

Session 940. Status: **accepted**. Recorded under the project owner's standing rule for this
tranche: a change to a shared crate made for `pdf-archive`'s sake is written down as an ADR when
its cost to the viewer is small, and asked as a `doc/questions/Q*` when it is not. This is the
first case, and the reason is one sentence: the new module is `pub` and nothing in the tree calls
it on any render path.

## What changed the premise

`doc/questions/Q51` had been open on buying the JPEG 2000 specifications. On 2026-09-07 the owner
bought **ISO/IEC 15444-1:2000, part 1, the core coding system**, which carries Annex I's JP2 box
structure and Annex A's codestream markers. Part 2, the extensions, is still not held; Q51 is now
open on that narrower question alone.

Three previous rounds had reached the same wall and correctly declined to guess at it. Five of the
seven rows of ISO 19005-2 §6.2.8.3 / ISO 19005-4 §6.2.7.3 come off `Check::Unchecked` with the
part in hand.

## The decision

**The reader goes in `pdf-model`, as `pdf_model::jpeg2000`, not in `pdf-archive`.**

`pdf-archive`'s own manifest states the rule this obeys, and it is worth quoting because it is the
whole argument:

> This crate adds no reader of its own, which is the whole design: a PDF/A requirement is a
> *question asked of readers this tree already has*, and one that needed a new reader would be a
> gap in `pdf-model` rather than a feature here.

So the question to answer was not "where is it convenient" but "is this a gap in the viewer". It
is, and the shape of the gap is more interesting than its existence.

**The tree could already decode JPEG 2000 and could not read it.** `pdf_model::image::decode_jpx`
hands the bytes to `pdf_sandbox`, which runs `hayro-jpeg2000` in a confined process and returns a
`Raster`: width, height, a component count, a `has_opacity` flag, one of five colour meanings, and
eight-bit samples. That is exactly what a rasteriser needs and it answers none of the five
questions ISO 19005 asks. How many `colr` boxes are there? What is each one's `METH`? Which of
them carries `APPROX` of 1? What precision does each component declare? Every one of those is
stated in the first hundred bytes of the data, in plain big-endian fields, and every one of them
was thrown away at the process boundary before anything reached this program.

The gap is therefore not "no decoder" — the crate documentation this round inherited said the
viewer could not render a JPX image, and that has been untrue since ADR 0161. The gap is that
**the only route to the data ran through a decode**, so a question about the header cost a
subprocess, a sample budget and an allocation, and still could not be answered.

`pdf_model::jpeg2000` is the cheap half of the same subject: `Headers::parse` walks I.4's boxes,
reads `ihdr` (I.5.3.1), `bpcc` (I.5.3.2), every `colr` (I.5.3.3) and `cdef` (I.5.3.6), notes
whether a `cmap` box exists (I.5.3.5), and reads the `SIZ` marker segment at the head of the first
codestream (A.5.1). It decodes nothing, starts no process, and allocates one small `Vec` per box
kind. Bare codestream data — which ISO 32000-2 §7.4.9 does not permit in a PDF but which producers
write — is read for its `SIZ` alone.

## The cost to the viewer, judged honestly

**Zero at run time, and that is checkable rather than argued.** Nothing in `pdf-model`,
`pdf-render`, `viewer-core` or either backend calls `jpeg2000::Headers::parse`; the only caller in
the tree is `pdf-archive`'s `for_each_jpeg2000`. `crate::image` is untouched, so the decode path,
its budget, its reduced-resolution retry (ADR 0321) and its channel reading (ADR 0464) are exactly
what they were.

The compile-time cost is one module of parsing over `&[u8]`, no new dependency, no new `pub use`
in `lib.rs`'s re-export list, and no new field on any type the viewer constructs.

Had the module instead added an eagerly-built index to a type `image.rs` already builds per image,
this would have been a `doc/questions/Q*` rather than an ADR. That is the line the owner's rule
draws and it is not close here.

## What the specification licensing forces, and what it does not

`doc/ISO_IEC 15444.pdf` is licensed to one reader, the way `doc/pdfa/`'s two parts are. So
**not one sentence of it appears in this tree**: `jpeg2000.rs` cites `I.5.3.3`, `A.5.1`,
`Table I-9` and the rest, and paraphrases in its own words. The single rustdoc blockquote in the
module is from ISO 32000-2 §7.4.9, which is quotable and which `tools/conformance`'s `quotations`
binary verifies character for character against `doc/md/`.

## The edition trap, which is the finding worth keeping

**ISO 19005 permits a `colr` box's `METH` to be 0x01, 0x02 or 0x03. ISO/IEC 15444-1:2000's
Table I-9 defines 1 and 2 and reserves everything else.** The third method comes from the later
extensions work. A validator that read the rule off part 1 would reject a value PDF/A explicitly
allows — and it would do so while looking, to a reader, as though it were being rigorous.

The same trap is set twice more in the same subclause and in the opposite direction:

- Table I-10 limits the first `colr` box's `EnumCS` to 16 and 17, and ISO 19005 names 12 (CMYK)
  as permitted and 19 (CIEJab) as forbidden — neither of which the first edition defines.
- I.5.3.3 says `APPROX` shall be zero and that readers shall ignore it, while ISO 19005 makes the
  value 0x01 *mean* "the best colour fidelity available" and requires exactly one specification to
  carry it.

In all three the rule implemented is **ISO 19005's**, and each constant in
`crates/pdf-archive/src/table/graphics.rs` says so above itself. The general lesson is the one
`CLAUDE.md` principle 5 already states from the other side: the edition a clause names is part of
the clause, and reading a requirement off an older edition of the document it defers to is a way
to be wrong while holding the right standard.

## What a JPX *renderer* would still need, and does not get from this

This module is not a step toward drawing anything, and listing what it omits is the honest way to
say so:

- **The codestream past `SIZ`.** `COD`, `QCD`, `COC`, `QCC`, `RGN`, `POC`, the tile-part headers,
  the packet headers and every coding pass. That is the decoder, and it stays `hayro-jpeg2000`'s
  behind the sandbox (ADR 0014, ADR 0161).
- **The palette.** `pclr` (I.5.3.4) and `cmap` (I.5.3.5) turn one component into several channels,
  which is the JP2 equivalent of an `Indexed` space. Only the *presence* of `cmap` is reported,
  because that alone decides whether a `cdef` index addresses a component; neither box's contents
  are read. `Headers::colour_channels` therefore counts channels before a palette expands them.
- **The restricted ICC profile's meaning.** A `METH` of 2 yields the profile's bytes, borrowed
  rather than copied, and `pdf_model::icc::Profile::parse` is what would turn them into a
  transform. Nothing here does that.
- **A colour space.** Knowing that `EnumCS` is 16 is not knowing what sRGB is; the mapping from an
  enumerated number to a `crate::colour::ColourSpace` is not written, and for the numbers part 1
  does not define it cannot be written from what this project holds.
- **Resolution levels and regions.** NOTE 2's four progressions are the decoder's, and ADR 0373
  measured why the location one is not used.

## The consequence in `pdf-archive`

Five rows moved from `Check::Unchecked` to `Check::Implemented`: the channel count, the one
best-marked colour specification, the `METH` value, the CIEJab exclusion, and the bit depth. The
veraPDF PDF/A-2b corpus's `6.2.8.3` directory goes from two agreed and five missed to seven agreed
and none missed, with `over` still zero on all six targets.

Two rows stay `Unchecked` with sharpened reasons. The JPX baseline feature set is defined in
ISO/IEC 15444-2:2004 M.9.2 and the subclause's closing sentence defers to that document as well,
so both sentences of that row need the part this project does not hold — `doc/questions/Q51`. The
device-colour row stays for a different reason and it is not a purchase: the rule turns on when an
image *effectively* uses a device colour space, and neither part says which enumerated colour space
that is. Numbers 16 and 17 are sRGB and an sRGB-nonlinearity greyscale, which are calibrated rather
than device.

## What this does not license

A `Check::Unchecked` row is still the right answer for a rule whose reading is not held, and the
five rows that moved did so because a specification was read, not because a corpus went green. The
five witnesses agree with the five rules; they did not choose them.
