# 2452 — A clip's own container says where it ends and which markers it names

Session 1809, the clause slot of batch 121 on §13.2.4.3's two owed items.

Status: implemented — `crates/pdf-model/src/media/container.rs` (the header reader),
`crates/pdf-model/src/media.rs` (`Walk::span`, `Walk::bound`, `Walk::timeline`, `read_offset`), tested by
`crates/pdf-model/tests/media_viability.rs::an_end_past_the_clips_own_length_is_the_end_and_has_no_effect_on_viability`
and `::a_marker_the_clip_lacks_is_ignored_and_one_it_names_inside_the_clip_is_not`, fuzzed by
`fuzz/fuzz_targets/media_container.rs`; the subtype reading of item 6 by
`::an_unrecognised_offset_subtype_makes_the_rendition_non_viable_from_either_dictionary`.

## Context

ADR 2431 left §13.2.4.3 `partial` on two rules that each end "there is no effect on viability": an
`/E` "beyond the end of the next-level media clip", where "the end value shall be used instead", and
a marker that "is not present in the underlying media clip", whose entry "shall be ignored". The tree
judged both non-viable, because the clip's length, frame count and marker table are in the media
data and nothing here read it. The brief asked whether reading them is admitted by `CLAUDE.md`'s
clause 13 entry.

## The entry's reading

It is admitted, and the owner's word is not needed (Q675 unused). The entry keeps in scope
"§13.2.2's viability computed and reported in the clause's own terms" and excludes "decoding or
presenting audio and video". A container's duration field, its frame count and its marker names
are the clause's own terms for these two rules, and reading them decodes no sample: the reader never
reaches a sound or a picture, only header fields and, for QuickTime's chapter list, the text samples
that hold the chapter titles. It is not the `mp4` demuxer ADR 2428 lists for a reopening of Tier C,
which would hand audio and video samples to decoders. ADRs 2392 and 2431 read the entry the same way
in the ledger's note before any reader existed.

## Decision

1. **A bounded reader of four container families**, `pdf_model::media::container`, under
   `forbid(unsafe_code)` and with its fuzz target from this commit: RIFF WAVE and AVI (the 1991
   RIFF document), AIFF and AIFF-C (Apple's 1.3), and the ISO base media file format with
   QuickTime's chapter tracks. Every size the file states is checked against the bytes holding it;
   a walk visits at most 4 096 boxes or chunks; a marker table past 4 096 names is refused rather
   than cut, because a name left unread would be judged absent. The types are chosen by Table
   285's `/CT`, so a clip of any other type is not decoded at all.
2. **An `/E` holds** where the next-level clip is the whole file — it begins at the file's start
   and ends at its end, by the same rules one level down — and the offset is at or past the clip's
   length: seconds against the container's duration, a frame against its picture track's sample
   count. A sound has no frames, so a frame offset over one is a value this player does not
   support (§13.2.4.3 NOTE 2).
3. **A marker holds** where the container's table does not name it (ignored, as the clause says),
   where it falls outside the next-level clip that is the whole file, or where it falls at the
   file's start for a `/B` or at or past its end for an `/E`. A named marker inside the clip is a
   place the player would have to be told.
4. **What is not known is said, and stays non-viable.** A container this program does not read
   (the corpus's MPEG, MP3, Flash and Windows Media clips among them), a file cut short, a decode
   with damage, and media data named by URL or by a file outside the document are each the reason
   in the report. The judgement fetches nothing and opens no path (principle 3). Whether such an
   `/E` reaches the end, or such a marker is present, is a fact nobody here knows, so it is a
   value the player cannot be shown to honour, which is NOTE 2's case.
5. **An embedded stream is decoded only when a section's offset needs it**, under the document's
   own limits, once per stream for the walk.
6. **ADR 2447's reading is taken for a section's offsets.** Tables 296 and 300 say "The rendition
   is non-viable if the interactive PDF processor does not recognise the value of this entry", and
   the sentence names the rendition, so a `/B` or `/E` stating an unrecognised `/S` makes it
   non-viable from a `BE` as from an `MH`, in those tables' words. An absent `/S` is no value, and
   stays an offset not honoured. A media duration's `/T` under the play parameters' `BE` is
   §13.2.5's code and is not changed here.

## Consequences

- §13.2.4.3 moves `partial` → `implemented`, and §13.2.4's aggregate with it. §13.2 stays `partial`
  on §13.2.6's rows.
- The census behind the brief's hypothesis found it wrong: no corpus document (pdf.js's 974,
  `doc/corpora`'s 503, the cache's 89 286) holds a media clip section, and the 25 that hold media
  clip data name `audio/mpeg`, `video/avi`, `application/x-shockwave-flash`, `video/mpeg`,
  `video/mp4`, `video/quicktime`, `video/x-ms-wmv` and `audio/aiff`, with no WAVE file. The reader
  covers AVI, AIFF and MP4/QuickTime because the census found them, and WAVE because §13.3's sound
  crosses as one. The rule is the standard's and the population is not evidence for it (trap 8).
- A later reader for another family (MPEG program streams, MP3's frame headers) is a new arm of
  `container::timeline` and changes no judgement code.
