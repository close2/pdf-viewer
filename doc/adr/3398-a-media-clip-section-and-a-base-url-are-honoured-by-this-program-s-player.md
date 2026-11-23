# 3398 — A media clip section and a base URL are honoured by this program's player

Session 2174. Status: accepted. Context: ADR 3350 (viability judged for the player a clip
reaches), ADR 2452 (offsets against the container's timeline), ADR 2447; ISO 32000-2 §13.2.4.2,
§13.2.4.3, Tables 287–289 and 296–300.

## Decision

1. **A chain of media clip sections ending in a clip this player plays is played here.**
   `Walk::played_here` follows the chain to its media clip data. Each section's Table 289 `/B` and
   `/E` are placed on its next-level clip's own section: a time offset "relative to the start of
   the next-level media clip"; an `/E` past that clip's end held to it; a marker at the place its
   container states, and ignored where it is absent or outside the next level's range, as the
   clause says. A beginning at or past the end is the clause's valid empty range.
2. **A frame offset is not honoured**: a sound's container counts no frames
   (`container::Timeline::frames`). In `MH`, or below a section whose `MH` states the same entry,
   the section is non-viable; in `BE` it is set aside and said.
3. **The section rides `Playing::section`** (`pdf_model::media::Section`), through
   `viewer-confined`'s wire, to `viewer_host::sound`, which cuts the converted frames to it before
   the passes and says the section in the window's sentence. The hand-off is judged as before.
4. **Table 287's `/BU` is honoured by this player, in `MH` and `BE`.** §13.2.4.2's base URL is
   used "in resolving any relative URLs found within the media data"; the player resolves none — a
   WAVE file names no URL it follows, and an MP4 file is refused unless every data reference says
   the media is in the file itself — so nothing it does is relative to any base. This is a reading
   of the clause, recorded as one.
