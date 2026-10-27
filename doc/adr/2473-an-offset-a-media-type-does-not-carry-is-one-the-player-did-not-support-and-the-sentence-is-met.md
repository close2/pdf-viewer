# 2473 — An offset a media type does not carry is one the player did not support, and §13.2.6.2's sentence is met

Session 1819, the clause slot of batch 122 on §13.2.6.2.

Status: implemented — `crates/pdf-model/src/media.rs` (`Walk::bound`, `Walk::timeline`), tested by
`crates/pdf-model/tests/media_viability.rs::an_offset_a_media_type_does_not_carry_is_a_value_the_player_cannot_honour`,
and for the URL case by `::an_end_past_the_clips_own_length_is_the_end_and_has_no_effect_on_viability`.

## Context

ADR 2452's note on §13.2.6.2 left one thing "to read for a move": whether a clip in a format the
container reader does not read, judged non-viable with its reason, meets the clause's second
sentence, "Different media types support different types of offsets", or leaves the row owed.

## The reading

The sentence states a fact about media types and contains no `shall`. The consequence it has for a
processor is printed elsewhere, in §13.2.4.3: a `B` or `E` in a section's `MH` "shall be honoured at
play-time in order for the media clip section to be considered viable", and its NOTE 2 says "The
entry cannot be honoured if its value was not viable or if the player did not support its value; for
example, the player did not support markers." Table 296's `/S` names the three kinds, and Tables
297 to 300 say what each requires. So what the sentence asks of a processor is that an offset of a
kind the clip's type does not carry, or the player does not support, be a value the entry cannot
be honoured with.

The code already does exactly that, for every case the container reader decides: a frame offset over
a sound ("its container states no frames for a frame offset to count"), a time offset over a clip
whose container states no duration, and an offset over a container this program does not read, a
file cut short, a damaged decode, a URL and a file outside the document, each the reason in the
report. For these last, whether the type carries the offset and whether an `/E` reaches the clip's
end are facts nobody here knows, and a player handed only the file plays all of it; the value
cannot be shown to be honoured, which is NOTE 2's case, and the rendition is non-viable with the
reason said rather than chosen on a guess (trap 5). In a `BE` the same offsets are ignored, as
"[i]f B or E entry in a BE dictionary cannot be supported, it may be ignored at play-time".

## Decision

§13.2.6.2 is `implemented`. The sentence is met by the judgement above and nothing is left in the
tree that the clause asks for; the seeking to the offset when the clip plays ("shall specify an
offset into a media object") is the player's, excluded by `CLAUDE.md`'s clause 13 entry. A reader for
another container family changes no judgement (ADR 2452), only how often the "not known" case fires.

## Consequences

- §13.2.6.2 `partial` → `implemented`; with §13.2.6.3, §13.2.6 and §13.2 become `implemented`
  as aggregates, and leave `doc/todo/65`'s list of aggregate headings.
- The test's controls — the `BE` forms and a still image — keep the rule from reading as "every
  unusual offset is non-viable".
