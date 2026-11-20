# 3349 — A rendition's RIFF WAVE file is read in the confined media worker

Session 2162. Status: accepted. Context: RFC 0015 section 5, ADRs 3297, 3341, 3350,
`doc/questions/A946`; ISO 32000-2 §13.2.4.2 and Table 285; Microsoft's and IBM's *Multimedia
Programming Interface and Data Specifications 1.0* (1991) and RFC 2361, neither held in `doc/`.

## Decision

1. **The header is read where the samples are.** A media clip's `/D` is a file, and its header is
   the document's bytes like the rest of it, so `pdf_media_worker::wave::layout` walks the RIFF
   chunks behind `Profile::Media` — the `fmt ` chunk's tag, channels, rate, block alignment and
   width, then the `data` chunk's offset and length — and the host reads none of it.
2. **Two messages, then the old ones.** A layout request (`FRAME_LAYOUT`) carries the file's length
   and at most its first `MAX_RUN_BYTES`; the reply (`FRAME_PLACED`) is a decode request's format
   and the samples' offset and length. The host refuses a reply that places the samples outside the
   file or names a format the decoder refuses, and kills that worker, as it does a sample count that
   disagrees with its request. The samples then cross as a sound object's do, in runs of whole
   frames, under the same budgets. `MediaWorker::decode_wave` is the whole route.
3. **One more encoding, no new sample format.** A WAVE PCM sample of sixteen bits or more is whole
   bytes, least significant first (`Encoding::Little`); eight-bit PCM is unsigned and is `Raw`;
   tags 6 and 7 are G.711's A-law and µ-law. A sample is decoded as its container — a twelve-bit
   sample in two bytes is the same `i32` as a sixteen-bit one whose low four bits are zero.
4. **Declined, each with a sentence**: `WAVE_FORMAT_EXTENSIBLE`, IEEE float and every other tag; a
   block alignment that disagrees with the channels and the width; a `data` chunk before the `fmt `
   chunk, which the RIFF form orders the other way; and a header past the first `MAX_RUN_BYTES`.
   A declined file is not played; the reason is the window's sentence.
5. **The fuzz target grows with the parser**: `media_sound` runs the worker's whole `answer` over
   its input as a layout request too, and asserts that the host's check accepts every honest
   placement; `fuzz/seed_media_sound.py` seeds ten WAVE files and the new encoding.

## Why the header bound

A header read in one exchange keeps the worker stateless between requests. Chunks before `fmt `
(`LIST`, `bext`, cover art) can in principle push it past 256 KiB; such a file is declined and
said, not partly read. The alternative — a reply that names the next offset to show — makes the
worker hold a half-read header across requests, which the one-request-one-answer wire avoids.

## Cost

About 300 lines in `pdf-media-worker` with its tests, two wire kinds, and a public `answer` for
the fuzz target. Nothing on the launch path: the worker still starts at the first sound.
