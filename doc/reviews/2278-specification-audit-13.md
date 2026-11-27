# Specification audit of ISO 32000-2 clause 13 (multimedia features) — round 2278

The project's twenty-third specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2271 ([`2271-specification-audit-12-10-annexes-k-l-o-q.md`](2271-specification-audit-12-10-annexes-k-l-o-q.md)
is the one before). Baseline: main at `492622d82`. Evidence:
[`2278-specification-audit-13/`](2278-specification-audit-13/) — `findings.json` (2038's schema)
and `expected-failures.json` (the failing test of each behaviour finding). The same entries are in
`2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. The `shall`s and `should`s of §13.1 to §13.5 (`doc/md/ISO_32000-2_sponsored_EC3.md` lines 14080
   to 14764) were read with the ledger's clause 13 rows and `doc/scope-ratifications.md`'s clause 13
   entry, which decides what is in scope: what a still page shows, the media file saved or handed
   to this machine's player at the reader's level, and audio and video this program decodes in
   `pdf-media-worker` (RFC 0015 sections 5 and 6, `doc/questions/A946`, `A1006`). §13.6 (3D) and
   §13.7 (rich media) were read for what the page shows instead and for the refusal by name.
   Errata: `doc/errata/site/docs/32000-2-2020/clause13.md` — Table 277's `/N` as UTF-16BE, Table
   285's `/CT` "should" made "shall" (a writer's; no writer here writes `/CT`), the duplicate
   TEMPNEVER sentence below Table 286 deleted (the table row keeps it), Table 293's `/BE` as best
   effort, §13.2.7.2.1's last paragraph moved; none touches a finding below. Table 305 was read in
   the printed standard (`doc/ISO_32000-2_sponsored_EC3.pdf`), whose `/E` row lists `ALaw`, which
   `doc/md/`'s extraction drops (ADR 2398).
2. **§13.2.2 at every dictionary that carries `MH` and `BE`**: the rendition's (Table 278, and
   Table 279's criteria inside it), the clip data's (Table 287), the section's (Table 289), the play
   parameters' (Table 291), the screen parameters' (Table 294, and Table 295 inside it), the player
   info's (Table 302). For each, the three answers an entry can give — present and recognised,
   present with a value not recognised, absent — in `MH` and in `BE`.
3. **Every route to a media object**: a rendition action and a screen annotation
   (`pdf_model::media::judge`), a sound annotation and a sound action (`media::sound`), a movie
   annotation and a movie action (`media::movie`), a movie's poster (`appearance::movie_poster`
   for a stream, `media::poster_movie` and `viewer_host::video::Posters` for `true`), and a 3D or
   rich media annotation's press (`viewer_core::interact`). **Every writer of one**: XFDF's
   `<sound>` import is the one writer of a sound object in the tree.
4. **The players**: `pdf-media-worker`'s WAVE and MP4 readers, `viewer_host::sound::convert`
   against §13.3's last sentence, `viewer_host::video::timeline` against §13.2.5's freeze and
   repeat rules, the stop on close in every host (`quorra`, GTK, Qt, the C ABI), and the
   interlaced H.264 hand-off said by name.

What holds, probed by reading: Table 277's and Table 284's `/S`; the selector's depth-first first
viable rendition and the non-viable selector that evaluates nothing below it; Table 286's
permissions and `/TF` through the restriction policy (`Operation::HandOverMedia`); §13.2.4.2's
`/D` condition; §13.2.4.3's offset rules including the deeper-entries-as-`MH` rule (through
`span`); Table 291's `MH`-over-`BE` and its `/F` default; Table 292's `/S`; Table 294's `/W`, `/B`
and `/O` per window type; §13.2.7.2.2's four conditions; §13.2.7.4.4's negative versions; §13.3's
bit packing, byte order, interleave and the four encodings in the reader; the conversion to the
device; §13.2.5's freeze of the last picture; every clip stopped when its document closes;
Table 306's `/Poster` in its three readings.

## Findings

Four behaviour gaps. In order of how much a person would lose:

- **2278-B03** — an XFDF `<sound encoding="alaw">` is written as a sound object with no `/E`,
  which Table 305 reads as `Raw`. The importer's comment believes `doc/md/`'s extraction of Table
  305; the printed table lists `ALaw`, and `pdf_model::media::Encoding` reads it. A-law codes are
  then played as unsigned samples: noise where the producer recorded speech.
- **2278-B02** — a movie this program plays itself ignores Table 306's `/Rotate`: "The number of
  degrees by which the movie shall be rotated clockwise relative to the page." The entry is said
  among the set-aside entries and the pictures stand at the page's orientation. ADR 3594 lists it
  as said, and the 13.4 row as `implemented` beside it.
- **2278-B04** — a press on a 3D annotation or on a rich media annotation says nothing. Table
  310's `/A` defaults to `XA`, activation by the user, and activation shows the artwork, which
  this program does not render (the clause 13 exclusion). The page shows the normal appearance,
  which §13.6.2 permits, and the exclusion is never said where a person asks for the artwork
  (trap 5: each arm of a click says something). The 13.6.2 row: "an annotation this program cannot
  activate stays in the state the clause makes its default" — and says nothing.
- **2278-B01** — a value of the wrong type in a rendition's must-honour criteria is read as met.
  §13.2.2: "an object shall be considered non-viable if its MH dictionary contains an unrecognised
  key or an unrecognised value for a recognised key", and §13.2.3.1 excepts only an unrecognised
  *entry* in `BE`. `MH << /C << /V 5 >> >>`, `/P /2.0`, `/L (en)` and `MH << /C 5 >>` are each
  viable: `criterion` returns met for a `/V` or `/P` that is no array, `language_criterion` logs
  and reads an `/L` that is no array as absent, and `criteria` skips a `/C` that is no dictionary.

Three documentation findings:

- **D01** — the 13.4 row is `implemented` while three of Table 307's and Table 306's statements are
  set aside for the movie this program plays: `/Rotate` (B02), `/Rate` below zero ("the movie
  shall be played backward") and `/Mode /Palindrome` (played forward repeatedly). Either the row
  says `partial` or the departures are argued as such; the clause entry does not exclude them.
- **D02** — Table 279's `/P` is refused with "this program plays none" of the multimedia
  constructs; it plays §13.2's renditions, sounds and movies. The refusal may stand on "fully
  supported" (the 13.2.5 row is `partial`), and its sentence should say so.
- **D03** — the rich media rows (13.7, 13.7.1, 13.7.2, 13.7.2.2 and its six, 13.7.2.3, 13.7.2.3.3 to 13.7.2.3.6)
  rest on "a media engine's runtime — rich media's, which no round has planned beside the video
  player", and 13.7.2.1's note says the playing of video "still excludes". The ratification says
  rich media "follow[s] the parts [it is] made of and carr[ies] no trigger of [its] own", and RFC
  0015 section 10.1 makes a `Sound` configuration Part 1 and a `Video` configuration Part 2, both
  built. The `Sound` and `Video` configurations are therefore unblocked; the rows may stay
  `out-of-scope` until a round builds them (the ratification's rule), but their reason is stale
  and no todo plans the five rounds RFC 0015 section 10.1 estimates.

Not filed: a clip handed to another program is not stopped when its document closes (§13.2.6.1
"All media clips that are being played … shall be stopped when the document is closed"); this
program cannot stop another program, and the row records the reading (ADRs 1961, 2376). Table
279's `/A`, `/C`, `/O`, `/S` and `/R` are always unmet because the program holds no such
preference and measures no bandwidth — a reading, not a gap. A poster image drawn from a
`/Poster` stream is not turned by `/Rotate`; the table makes the rotation the movie's, and the
poster is drawn at a placement the clause leaves to the reader (ADR 1561).

## Ledger rows contradicted

Each stays as it is until the fix round, which re-reads it: `13.2.2` and `13.2.3.1` (B01), `13.4`
(B02, D01), `13.3` (B03, whose writer is XFDF's import), `13.6.2` and `13.7.2.1` (B04, and D03 for
13.7.2.1), `13.2.3.1` (D02), the rich media `out-of-scope` rows (D03). All named rows are
`implemented` but the rich media ones.

## Suggested fix groups

1. **§13.2.2's unrecognised values in a rendition** — B01 in `pdf-model/src/media.rs`: `criteria`
   and `criterion` take the `MH` flag for every value-type check, so a `/V`, `/P` or `/L` that is
   no array and a `/C` that is no dictionary are reasons in `MH` and ignored in `BE`; D02's
   sentence with it.
2. **The movie this program plays** — B02 and D01: `/Rotate` carried on `Playing` (or `Clip`) and
   applied by `viewer_host::video` where it places the pictures and the poster it decodes for
   `/Poster true`; `/Rate` below zero and `Palindrome` built or argued as departures, and the row
   restated.
3. **XFDF's sound** — B03: `alaw` writes `/E /ALaw`, the comment restated against the printed
   table.
4. **Excluded artwork, said** — B04: `exhibited` (or the press path after it) says, for `3D` and
   `RichMedia`, that the artwork is not rendered and the normal appearance stays; D03's rows
   restated and rich media's `Sound` and `Video` configurations planned as a todo.
