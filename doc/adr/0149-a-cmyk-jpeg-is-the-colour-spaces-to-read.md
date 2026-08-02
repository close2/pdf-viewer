# ADR 0149 — A CMYK JPEG is the colour space's to read

Status: accepted, 2026-08-02. Session 178. The second defect the ambiguous ranking named, and a
colour conversion this project forbids that a dependency was doing anyway.

## What it looked like

`cmykjpeg.pdf` is one photograph on one page: a dog on a beach, 200×150, `DeviceCMYK`,
`DCTDecode`. Four reference renderers draw the photograph. **We drew it black**, with the dog a
smear of bright noise, and the corpus gate reported nothing because nothing was missing.

The oracle's ambiguous ranking put it second, at 30.19 bounds from the *nearest* reference against
30.23 from the furthest — two numbers that close mean every renderer disagrees with us rather than
with each other. The verdict was `ambiguous` and could not have been anything else: this is a page
of `DeviceCMYK`, where `mupdf` and `ghostscript` share an ICC profile and `poppler` does not (ADR
0048), so no two references agree tightly enough to contradict anybody. A page can be visibly,
grossly wrong inside that verdict, and this one was.

## The cause

`decode_jpeg` asks `zune-jpeg` for a raster and takes what it gets. The default output colour
space is RGB, and for a four-component codestream that means **`zune-jpeg` performs a CMYK to RGB
conversion of its own**: `blinn_8x8(c, k)`, which is `(1 − C)(1 − K)` computed on samples it
assumes are stored *inverted* — the convention a standalone Adobe CMYK JPEG follows.

`cmykjpeg.pdf`'s samples are not inverted. Its first is `(122, 55, 14, 1)`: a little cyan, less
magenta, almost no yellow or black, which is sky. Read as inverted it is `(133, 200, 241, 254)` —
nearly full ink in every channel, which is black. Both `ImageMagick` and PIL report the second,
because both apply the standalone convention on a file carrying the Adobe APP14 marker; all four
reference renderers render the first.

Two things were wrong with this, and only one of them is about this file.

**It is a second route from colour to pixels.** Trap 6 in the handover is categorical:
`ColourSpace::to_rgb` is the only place a colour becomes RGB, and `colour::xyz_d50_to_srgb` the
only place an XYZ becomes a pixel. That rule exists because three `DeviceCMYK` conversions once
lived in this tree and disagreed. A fourth had been living in a *dependency* the whole time,
reachable by any four-component JPEG, and no test in `colour_paths.rs` could see it because every
fixture there states its samples as hex rather than as a codestream.

**And inside a PDF the polarity of a sample is not the marker's to state.** §8.9.5.2's `/Decode`
array is what says what a sample means, and Table 88's default for `DeviceCMYK` is
`[0 1 0 1 0 1 0 1]` — the identity. §7.4.8 does defer to Adobe Technical Note #5116, and the
deferral is normative — "PDF DCT encoding shall exactly follow all those rules established by
Adobe for the PostScript language" — but what it defers is which *markers* to honour and whether
to undo the YCbCr or YCCK transform, both of which are about how components were *coded*. An
inversion is about what a component *means*, and PDF has an entry for that. A file that stores
inverted CMYK says so with `/Decode [1 0 1 0 1 0 1 0]`, which this tree has read since the
twenty-fifth session and which `issue7406.pdf` exercises.

## What changed

`decode_jpeg` asks for CMYK output when the codestream is CMYK, and hands the four components on
untouched. `apply_decode_to_channels` applies §8.9.5.2 to four channels instead of three.
`convert_channels` gains a four-component arm, `convert_four`, which is `convert_three` with one
more component and the alpha byte restored — the fourth component travels in it until then.

`DeviceCMYK` stops being a space `convert_channels` skips. It skipped it under the heading "a
device space is what the decoder already produced", which was true of grey and RGB and had never
been true of CMYK.

The mismatch arm is now stated as a pair, `(space components, raster components)`, and it is
deliberately permissive in one direction: a greyscale JPEG is delivered as three equal channels,
so a one-component space reads the first of them, exactly as before. Making the check symmetric
was tried first and cost two corpus documents a new report for a file that was not malformed —
measured, reverted, and worth recording as the shape of an over-strict check.

YCCK is left alone. `zune-jpeg` will not convert YCCK to CMYK, only to RGB, so a YCCK codestream
still takes the decoder's own path; no corpus document exercises it, and inventing a conversion
for a case nobody writes is the speculative work `CLAUDE.md` forbids.

## What it cost

Every gate re-run. The corpus is unchanged — 974 documents, 0 unopenable, 74 incomplete, the same
74. The text gate is unchanged at 98.2%. The dates gate is unchanged. No oracle verdict moved.

What moved is the page: mean 6.28 → **0.40**, worst tile 151.17 → **12.03**, structural similarity
0.9306 → **0.9885**. It stays `ambiguous`, and now for the honest reason — the residue is the
`DeviceCMYK` conversion the references themselves disagree about, and our distance from each of
them is smaller than `poppler`'s distance from either. `AMBIGUOUS_DEVICE_CMYK_CONVERSION` carries
the six pairwise numbers.

`colour_paths.rs` gains the fourth route it was missing: the same file's first sample, asserted to
be what `ColourSpace::to_rgb` makes of it. Checked by breaking — with the CMYK request removed it
fails.
