# 3350 — Viability is judged for the player a clip reaches, and a WAVE clip is played here

Session 2162. Status: accepted. Context: ADRs 1960, 3301, 3341, 3349; ISO 32000-2 §13.2.2,
§13.2.3.3, §13.2.4.2, §13.2.5, Tables 285, 286, 290 to 292; `doc/ui-boundary.md`.

## Decision

1. **A host states its players.** `Command::PlaysHere(bool)` says whether the host plays an
   embedded RIFF WAVE clip in its own window; `quorra`, GTK and Qt send `true` at launch, every
   other host says nothing and keeps the hand-off alone. It is a value only a host has — the
   boundary's test for a new message — and it reaches the judgement as
   `pdf_model::media::Processor::players` (`Players::HandOff` or `Players::AlsoHere`), held in the
   view state on `Command::Separations`' rule.
2. **The player is chosen per clip, before the judgement.** Where the host plays here and a media
   rendition's `/C` is media clip data holding an embedded file whose `/CT` names a WAVE type
   (RFC 2361's `audio/vnd.wave` and the three names files carry), the rendition is judged for this
   program's player; every other clip is judged for the hand-off as ADR 1960 states.
3. **What this player honours of Table 291** (`Playing::of_play_parameters`), MH's instance over
   BE's as §13.2.2 orders: `/V` a level, `/D`'s three durations, `/RC`, `/A` (with ADR 3351's resume
   control), and `/C` false. `/F` is honoured for an aural clip: the entry is about "a visual media
   type that does not exactly fit the rectangle in which it plays", and a sound fits no rectangle.
   **This reading is a choice** the clause does not make for us. `/C` true asks for "a
   player-specific controller user interface", which this player does not show — a window's keys
   are not one — so it stays non-viable in MH. An unrecognised MH entry is non-viable; a BE entry
   this player cannot honour is set aside and said.
4. **§13.2.3.3 then falls to the next rendition** where an MH entry is not honoured, exactly as for
   the hand-off: the walk is unchanged, only the question asked of Table 291 is.
5. **A clip judged for this player is never handed on.** `Clip::played_here` marks it; viewer-core
   puts the whole file in `Event::Media`'s `sound` as `Recording::Wave` with the parameters, and the
   window's `handing_here` plays it at the `MediaFiles` level's play-here answer. Table 286's `/TF`
   asks nothing of it: this player writes no temporary file. `SoundData` became a `Recording` and
   the parameters, so that a sound object's samples and a WAVE file cross one channel.

## What stays as it was

Table 294's screen parameters and Table 287's `/BU` in MH stay non-viable for both players, and a
media clip section over a WAVE clip is the hand-off's. Each is a later question with its own clause
reading; none is decided here by default.

## Cost

A command, an enum, a field on `Clip`, a reading of Table 291 of about 150 lines, and
`SoundData`'s shape — every consumer failed to compile and was changed, which is what the boundary
asks of a changed value.
