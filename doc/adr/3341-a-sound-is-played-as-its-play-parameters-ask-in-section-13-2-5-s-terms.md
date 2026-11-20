# 3341 — A sound is played as its play parameters ask, in §13.2.5's terms

Session 2160. Status: accepted. Context: RFC 0015 section 5 (step 7), ADRs 1960, 2382, 3297, 3301,
3317, 3329, `doc/questions/A946`.

## Decision

1. **One vocabulary.** `pdf_model::media::Playing` holds §13.2.5's play parameters — Table 291's
   `/V` as a level (a fraction of the recorded level), Table 292's simple duration (intrinsic,
   infinite, or seconds), `/RC` and `/A` — with Table 212's `/Synchronous` and `/Mix` beside them.
   Every default is the tables' own. It travels on `pdf_model::media::Clip::playing`, on
   `viewer_core::SoundData::playing` and across `viewer-confined`'s wire (each number held finite
   and non-negative there, the confined side being the less trusted one).
2. **Table 212 is read into it** (`Playing::of_sound_action`): `/Volume` is the level; `/Repeat`
   true is a repeat count of zero, "Zero means repeat forever"; `/Repeat` *present* sets
   `/Synchronous` aside, because the table says "If this entry is present" and not "if true". A
   value of the wrong type keeps its default and is said.
3. **The negative half of `/Volume` is silence. This is a choice**: Table 212 gives the range
   "-1.0 to 1.0" and a default and nothing about what a number in it means. Read as a scale on
   samples, a negative number would only invert their sign, which no ear hears as a volume; read as
   a level, the half below zero has no meaning. The program plays it silent and says so in a note.
   A number outside the range is held to it and said.
4. **The player honours it** (`viewer_host::sound`): every converted sample scaled by the level and
   held to full scale (`/V` may ask for more than was recorded); the sound cut into passes of the
   simple duration — cut where shorter than the sound, frozen silent where longer ("the media is
   logically frozen but shall not continue to produce sound"), counted by `/RC` with a fractional
   last pass, for ever at zero until Escape, a second sound or the document's close stops it; and
   held at its start under `/A` false until `Player::resume`. A pass of no length ends the sound
   even under a count of zero: nothing repeated for ever is nothing, and a loop on it would spin.
   The sentence that says a sound plays names every parameter that is not its default.
5. **`/Mix` true is not mixed**, and the sentence says so: the device takes one sound, so the earlier
   one is stopped as at `/Mix` false. Mixing needs the earlier sound's remaining samples, which the
   media clock (ADR 3342) now locates; it is left for a later round.

## Why Table 291's own dictionaries are not read into it yet

The brief's premise was that §13.2.5's `MH` entries become honourable now that this program is a
player. It does not hold yet: this player plays §13.3's sound objects only, and those are reached by
Table 212's action and §12.5.6.16's annotation, never by a rendition — Table 285's `/D` is "\[a]
full file specification or form XObject". A rendition's audio is a file handed to this machine's
player, which is told nothing, so ADR 1960's judgement (every `MH` entry non-viable) still answers
for it exactly. The step that changes that is decoding an embedded audio file (RIFF WAVE first,
§13.3's own encodings inside it) in the confined media worker; then Table 291's `MH` and `BE` are
read into `Playing` and the viability judgement learns which player a clip will reach.

## Cost

About 150 lines in `pdf-model`, 120 of arithmetic in `viewer-host`, a field through `viewer-core`
and the wire. `SoundData`, `Played`, `Handed`, `Handing` and `Asked` lose `Eq` (a level is an
`f64`); nothing compared them but tests, which `PartialEq` serves.
