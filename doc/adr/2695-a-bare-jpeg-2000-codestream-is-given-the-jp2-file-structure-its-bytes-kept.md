# 2695 — A bare JPEG 2000 codestream is given the JP2 file structure, its bytes kept

Session 1940, a single archive round.
Status: accepted and built: `pdf-jpx-write`'s `around.rs` (`around`, `is_bare_codestream`, three
`Error` variants); `pdf-transform`'s `archive/transcode.rs` (`given_its_file_structure`,
`TranscodedImage::codestream_kept`); `crates/pdf-transform/tests/archive/kept_codestream.rs`.
Context: ADR 1400 (the transcode in the stated space), ADR 1412 (opacity), ADR 2484 (the deep
image's JP2 file), ADR 2490 (R13), ADR 2694 (which profiles answer the site).
Clauses: ISO 32000-2 §7.4.9; ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3;
ITU-T T.801 M.9.2; ISO/IEC 15444-1 A.5.1 and Annex I (cited and paraphrased).

## The premise, checked

R13's three documents, `bug_jpx.pdf`, `issue5567.pdf` and `red_stamp.pdf` (pdf.js), each hold one
`JPXDecode` image whose stream opens `FF 4F FF 51`: an ISO/IEC 15444-1 codestream with no box around
it. `bug_jpx` is 128 × 64, one 8-bit component under `/DeviceGray`; the other two are three 8-bit
components under `/DeviceRGB`. None states `SMaskInData`. §7.4.9 has the filter "expect to read a
full JPX file structure", so each is outside ITU-T T.801 M.9.2's baseline for being no JPX file at
all — the validator's "a bare codestream" — and for nothing in the codestream. Every dictionary
states its `/ColorSpace`, so all three are ADR 1400's answered shape; the transcode was built and
only the two shipped profiles left it at `stop` (ADR 2694).

## Decision

1. Before any transcode, a bare codestream is given the JP2 file structure of ISO/IEC 15444-1
   Annex I, the codestream kept byte for byte as its last box: `pdf_jpx_write::around` reads the
   image header and, where depths or signs differ, the bits-per-component box out of the
   codestream's own `SIZ`, and writes the boxes `pdf_jpx_write::jp2` already writes around its
   own codestreams. Only the stream's `/Length` changes in the dictionary.
2. The colour specification box enumerates greyscale for one channel and sRGB for three. §7.4.9
   has a stated `/ColorSpace` determine the samples' meaning and the data's specifications be
   ignored, so the box decides nothing a reader draws; it is written only beside a stated space.
3. It is refused, and the image goes to the transcode as before, where the codestream's `Rsiz`
   asks for ISO/IEC 15444-2's extensions or a `CAP` marker's capabilities (bits 15 and 14; a JP2
   file may not hold them and M.9.2.3's baseline does not admit them), where colour channels
   differ in depth (sections 6.2.8.3 and 6.2.7.3 admit one depth), where the channel count is not
   one or three, where an opacity channel is stated, and where the file does not decode back to
   the source's samples, grid and depths (ADR 0973's proof, the deep path's `reads_back_as`).
4. A dictionary stating no `/ColorSpace` keeps the refusal it had: a bare codestream then has no
   colour specification at all and a reader draws it by its channel count, and any box written
   for it would be this converter naming the space.
5. Nothing is recorded in `xmpMM:History`: a restatement that costs nothing records nothing
   (ADR 2693). The report's row says `codestream_kept`.

## Why not the transcode, or the baseline writer, here

Both decode the producer's codestream and write this tree's decoder's output, so ADR 1400 section
4's two costs follow — the file grows (`issue5567.pdf`'s 2.1 MB codestream would become a Flate
copy of 5 MB of samples) and a decoder fault is kept as the picture. Wrapping pays neither: the
file grows by the boxes, under a hundred bytes, and any decoder reads the producer's codestream.

## What it costs

A wrapped codestream is still whatever ISO/IEC 15444-1 coding its producer chose, so a decoder
that fails on it fails on the archive's copy too; that is the producer's picture, not a loss.
