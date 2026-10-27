# 2489 — An MPEG-2 program stream states no length and no offset, and MPEG-1 and MP3 stay unread for want of their texts

Session 1827, the clause slot of batch 123, the third arm and the census.

Status: implemented — `crates/pdf-model/src/media/container/program_stream.rs` and
`container::unread` in `crates/pdf-model/src/media/container.rs`, `media::clip_timeline` in
`crates/pdf-model/src/media.rs`, the census in `crates/pdf-model/examples/media_clip_census.rs`,
tested by the program stream unit tests and
`crates/pdf-model/tests/media_viability.rs::a_flash_a_windows_media_and_a_program_stream_clip_are_judged_by_their_own_headers`,
fuzzed by `fuzz/fuzz_targets/media_container.rs` (type 6 for the stream, 7 for the unread audio type).

## Context

The slot named ITU-T H.222.0 for an MPEG-2 program stream and asked whether the MPEG-1 texts (ISO/IEC
11172-1 systems, 11172-3 audio) can be had. ADR 2488 holds the SWF and ASF arms and the reading of
the clause 13 entry; this ADR is the one that does not read a length.

## The specifications

- **H.222.0** is free from the ITU: ITU-T Rec. H.222.0 (07/95), identical in text to ISO/IEC
  13818-1:1996, from https://www.itu.int/rec/T-REC-H.222.0-199507-S/en. It is the **1995 edition**,
  superseded by later editions, which were not read; the tree reads only the pack header of clause
  2.5.3 (Table 2-33) and states that edition. It is not committed.
- **ISO/IEC 11172-1, 11172-3 and 13818-3** are sold by ISO and by no free route this round found
  (the standards' abstract pages and a purchase page only). The tree does not hold them, so the MPEG-1
  system stream and MPEG audio (`audio/mpeg`, MP3) stay "unread container". A frame header was not
  taken from a secondary description of the format as though it were the standard (`CLAUDE.md`
  principle 5): a reading from memory or from another implementation is evidence about the
  standard, not the standard.

## Decision

1. **The program stream arm reads the first pack header** (clause 2.5.3, Table 2-33): the
   `pack_start_code`, the `01` bits, the three-part system clock reference with its marker bits, the
   22-bit `program_mux_rate` (zero forbidden) and the stuffing length. A file that does not carry the
   bits and marker bits Table 2-33 fixes is not the container its type names and is refused,
   whatever system layer it is. H.222.0 describes no other pack header, so none is named: an MPEG-1
   system stream is recognised by no rule here and falls under the same refusal.
2. **A program stream states no length and no named offset.** The system clock reference is a time of
   delivery and not of presentation; presentation times are in PES packet headers, and the last
   picture's duration is in the video bitstream (ISO/IEC 13818-2, not held). H.222.0 defines no
   mechanism that names an offset in the stream, so a marker over one is absent, which §13.2.4.3
   ignores with no effect on viability. The reader therefore returns an empty timeline: a recognised
   program stream whose length nobody here can state.
3. **`container::unread(content_type)` names why a type is unread**, so the report says which
   standard the tree lacks for MPEG audio instead of a bare "reads no such file".
4. **`media::clip_timeline(document, data)` is the census door**: the function the judgement runs,
   made public so that a command counts what the judgement reads rather than a copy of it.

## Census

`media_clip_census` opens every object of every document (a direct dictionary nested in a rendition
included, because renditions inline their media clip data). Its input is the whole population, none
of it narrowed: every PDF under `doc/pdf.js`, `doc/corpora`, `doc/corpora-own` and
`doc/veraPDF-corpus` (4 389 paths) and under the whole of `corpus-cache` (89 286 paths from the
`openpreserve`, `safedocs` and `tika-issue-tracker` collections), 93 675 in all, built by `find -L`
over those directories as the example's documentation spells. Every path lands in one count: 93 080
opened, 172 skipped for size, 423 unreadable (not read, or not a document `pdf_syntax` opens), so the
denominator adds up. It found 28 documents with media clip data and 69 clips.

| | read | unread container | not judged |
|---|---|---|---|
| the SWF, ASF and program stream arms disabled | 11 | 39 | 19 |
| with them | 22 | 21 | 26 |

- **read (22)**: 10 Flash, 1 Windows Media, 3 AIFF, 5 QuickTime, 2 AVI, 1 MP4.
- **unread container (21)**: 19 `audio/mpeg` clips in 3 documents, for the missing texts, and 2
  clips whose `/CT` is empty or absent.
- **not judged (26)**: the media data of 16 AVI and all 7 `video/mpeg` clips is a URL, which the
  renderer does not fetch; 3 MP4 clips are cut short.
- **No corpus clip exercises the program stream arm**: every `video/mpeg` clip is named by a URL, so
  the arm and its refusal stand on their tests and the fuzz target alone. Whether
  a real `video/mpeg` clip embedded in a file is a program stream or an MPEG-1 stream is not
  answered by this population.

## Premises that did not hold

- The earlier census, an indirect-object walk, found 15 documents; renditions inline their media clip
  data in a direct dictionary (`/R << /C << /CT ... >> >>`), so it missed 13. A census over objects
  visits the dictionaries nested in a direct dictionary and not only the indirect objects.
- The brief named ASF revision 01.20.06; the revision that can be had is 01.20.03 (ADR 2488).
- The brief expected most of the 25 documents to judge viable by their headers. 22 of 69 clips are
  read; most of the rest name a URL, which is not a container question.

## Consequences

- The MP3 and MPEG-1 clips stay unread until ISO/IEC 11172 and 13818-3 are held; reopening is a new
  arm of `container::timeline` and changes no judgement code.
- A later edition of H.222.0 is read against clause 2.5.3 before the arm cites it.
