# 2488 — An SWF and an ASF clip are judged by the length and the names their own headers state

Session 1827, the clause slot of batch 123 on §13.2.4.3's end and marker rules over the two
containers ADR 2452 left unread whose specifications can be had.

Status: implemented — `crates/pdf-model/src/media/container/swf.rs` and `asf.rs`, routed from
`crates/pdf-model/src/media/container.rs` (`Family::Swf`, `Family::Asf`), tested by their unit tests and by
`crates/pdf-model/tests/media_viability.rs::a_flash_a_windows_media_and_a_program_stream_clip_are_judged_by_their_own_headers`,
fuzzed by `fuzz/fuzz_targets/media_container.rs` (types 4 and 5), counted by
`crates/pdf-model/examples/media_clip_census.rs`.

## Context

ADR 2452 read four container families and left the clips of every other type "unread container", the
census naming `application/x-shockwave-flash`, `video/x-ms-wmv`, `video/mpeg` and `audio/mpeg`. The
slot asked for each of those judged by its own published specification where the text can be had,
and for a format whose text the tree does not hold to stay unread (`CLAUDE.md` principle 5). ADR 2452's
reading of the clause 13 entry carries over unchanged: a header field is §13.2.2's viability computed
in the clause's own terms, and no sample is decoded. The judgement code is untouched; each reader is
an arm of `container::timeline` (ADR 2452 item 1). ADR 2489 is the third arm and the formats that
stay unread.

## The specifications

None is committed. Each is cited by section and none is quoted, because none is in `doc/md/` for the
quotation checker to hold it to and two are copyrighted:

- **SWF**: Adobe, *SWF File Format Specification*, version 19, as mirrored at
  https://open-flash.github.io/mirrors/swf-spec-19.pdf. Adobe's copyright forbids copying, so the
  tree links it.
- **ASF**: Microsoft, *Advanced Systems Format (ASF) Specification*, **revision 01.20.03** of
  December 2004, the HTML Microsoft published at
  http://msdn.microsoft.com/en-us/library/bb643323.aspx, read from the Internet Archive's copy of
  20 April 2014 (http://web.archive.org/web/20140419205228/http://msdn.microsoft.com/en-us/library/bb643323.aspx).
  The brief named revision 01.20.06; the text that can be had is 01.20.03, and the readings below
  are the ones that revision states.

## Decision

1. **SWF.** The header holds an 8.8 fixed-point frame rate and a 16-bit frame count; the length is
   the count over the rate and the picture count is the count (the header chapter). The named
   offsets are the main timeline's frame labels: the `FrameLabel` tag (type 43) and the
   `DefineSceneAndFrameLabelData` tag (type 86), each placed at the frame that the `ShowFrame` tags
   before it number. A tag inside a `DefineSprite` names the sprite's own timeline, so the walk steps
   over a sprite by its stated length. `CWS` (zlib) is inflated under the length the header states
   and refused when it inflates to another; `ZWS` (LZMA) is refused because the tree has no decoder
   for it. A rate of zero states no length and places no label. A label in a file older than
   version 6 that is not ASCII is refused, because those versions leave the encoding unstated.
   At most 131 072 tags are walked and the marker table is bounded by the container module's
   bounds.
2. **ASF.** The Header Object opens every file and is the check on the GUID byte layout: the
   specification gives GUIDs as text and says the format is little-endian without drawing the
   sixteen bytes, so they are read as the Windows `GUID` the text spells, and a file whose first
   sixteen bytes are not that reading of the Header Object's GUID (section 10.1) is refused. The
   File Properties Object (section 3.2) states the play duration in 100 ns units and the preroll in
   milliseconds; the length is the first less the second, a broadcast file states none, and a preroll
   past the duration states none. The Marker Object (section 3.7) names jump points with a
   presentation time and a description.
3. **A preroll beside markers is refused.** The specification does not say whether a marker's
   presentation time carries the preroll that payload times carry, so a file that names markers and
   states a nonzero preroll is judged "cannot be placed", and a marker would otherwise be judged
   inside or outside the clip by a guess. A file with markers and no preroll, or a preroll and no
   markers, is read.

## Census

`cargo run --release -p pdf-model --example media_clip_census -- @list` over the population
(93 080 documents opened, 172 skipped for size, 423 unreadable of 93 675) counts clips by the type they name. With the SWF and
ASF arms disabled, 10 Flash clips and 1 Windows Media clip were "unread container"; with them, all 11
are read: the 6 documents' Flash clips state a length and a frame count and one names a marker, and
the one Windows Media document (the corpus's GHOSTSCRIPT-695445-0) states 38.093 s of play and 8 000
ms of preroll, so 30.093 s, which was read from the file's bytes by hand as well. ADR 2489 carries
the whole table.

## Consequences

- An `/E` past a Flash or Windows Media clip's length is the end and has no effect on viability, and
  a marker the clip does not name is ignored, where both were judged "cannot place" before.
- A marker over an ASF file whose preroll is nonzero stays unplaceable until a text
  that says how the preroll applies to markers is held.
- The GUID layout is an inference verified on the corpus file and on the Header Object's own GUID of
  every file; a later revision of the specification that draws the bytes would confirm or change it.
