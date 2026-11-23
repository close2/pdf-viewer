# 3460 — The MP4 reader is checked against ISO/IEC 14496-12:2012

Session 2189. Status: accepted. Context: A988 (the fourth edition placed at
`doc/ISO-IEC-14496-12-2012.pdf`, SHA-256 `96d29351…a96`); ADRs 3369 and 3383, which recorded the text
as not held.

## Decision

`pdf_media_worker::mp4`'s restatements were read against the fourth edition, box by box: the box
header and full box (4.2), `moov` and `mvhd` (8.2.1, 8.2.2), `trak` and `tkhd` (8.3.1, 8.3.2),
`mdia`, `mdhd`, `hdlr`, `minf` (8.4.1–8.4.4), `stbl` and `stsd` with `AudioSampleEntry` (8.5.1,
8.5.2.2), `stts` (8.6.1.2), `edts` and `elst` (8.6.5, 8.6.6), `dref` (8.7.2), `stsz` and `stz2`
(8.7.3), `stsc` (8.7.4) and `stco` and `co64` (8.7.5). Every field width, version rule, entry layout
and time scale matches; every clause number the module cites is the fourth edition's. No reading
is contradicted, so no parse changes and no new test is owed.

Three sentences changed, in the module's current-reason places (the records keep their text):

- the module and `presentation` now cite the held edition instead of saying it is not held;
- a size of 0 is the last box of the file, running to its end (4.2); inside another box, reading it
  to the end of what holds it is stated as this reader's choice;
- `AudioSampleEntryV1` belongs to later editions; the fourth holds those bytes reserved as zero
  (8.5.2.2).

ADR 3383's zero segment duration stays a choice, now with its ground stated: 8.6.6.1 gives a zero
duration a meaning only in the empty initial movie of a fragmented file. The edit-list reading of
ADR 3383 — segment durations in the movie header's time scale, media times in the media's, −1 an
empty edit (8.6.6.3) — is the text's. Two declines remain declines: 4.2 says boxes of an
unrecognised version "shall be ignored and skipped", and this reader refuses an `mvhd`, `mdhd` or
`elst` of a version it does not know rather than playing a track whose times it cannot read.
ISO/IEC 14496-1, -3 and -14 stay unheld, and the `esds` layout stays paraphrased.
