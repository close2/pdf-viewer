# Q631 — May a movie, a sound and a rich media annotation's assets be handed over as a rendition's clip is, and has sound playing's demand come?

Asked by round 1768, which re-read the ledger's `inapplicable` and `out-of-scope` rows for scope
(`doc/reviews/1768-the-inapplicable-and-out-of-scope-rows-re-read-for-scope.md`, finding 4).

## The rows, and the reason they record

- **§12.5.6.17** (movie annotations) and **§12.6.4.10** (movie actions): `out-of-scope`, because the
  one thing either asks of a processor is that the movie be played, which the clause 13 entry
  excludes. The poster is §13.4's and is drawn.
- **§12.6.4.9** (sound actions) and **§13.3** (sounds): `out-of-scope` for the same reason; a sound
  annotation's icon is §12.5.6.16's and is drawn.
- **§13.7.2.3.2** (the assets name tree): `out-of-scope` as part of the rich media engine.

## What has changed

`CLAUDE.md`'s clause 13 entry, amended by `A254` on 2026-10-08, puts "the media file saved or handed
to this machine's own player" in scope, and RFC 0009 section 3.5's Tier A, which `A254` adopted, names
§13.4's movie dictionary and §13.3's sound object among what `pdf_model::media` reads. ADR 1961 built
the hand-off for a rendition's clip (`Event::Media`, at `Operation::HandOverMedia`, `ask` by default),
and §12.6.4.14 is `implemented` on it. Nothing hands a movie or a sound over: `media.rs` reads neither,
and `action.rs` still answers `Sound` and `Movie` with "clause 13's multimedia, excluded".

## What moving them would take

- **A movie** (Table 306's `/F`, a file specification like Table 285's `/D`): a movie annotation's
  activation and a movie action's `Play` go through the same hand-off; `Stop`, `Pause` and `Resume`
  act on nothing playing, as a rendition's do. One round, in `pdf-model` and `viewer-core`; the three
  windows already show the hand-off.
- **A sound with `/F`**: Table 305's sound is then a self-describing file (AIFF, RIFF, snd), handed
  over the same way.
- **A sound with embedded samples**: no file exists. Either this program writes a RIFF header over the
  samples — µ-law and A-law go across unchanged, while 8-bit signed and big-endian 16-bit samples are
  re-encoded on the way, which is close to decoding — or it plays them itself, RFC 0009's Tier B
  (a sample loop in the confined worker, PCM over the wire, `cpal` in the hosts), which `A254` put
  "on demand": the first document whose sound annotation has no `/AP` or whose `/S /Sound` action a
  person clicks. No corpus document has shown either (§12.6.4.9's row; RFC 0009 section 3.5).
- **Rich media assets**: §7.11.4 embedded files reachable only from a `RichMediaContent` dictionary,
  which `attachment::attachments` does not walk; listing them makes them savable under the existing
  `EmbeddedDocuments` level. Handing a Flash asset to a player is pointless; saving it is not.

## The question

1. Does the amended entry's "the media file" cover a movie's `/F`, a sound object's `/F` and a rich
   media annotation's assets, so that these are built under it?
2. For a sound with embedded samples: a RIFF file written for the hand-off, Tier B, or neither until
   the demand comes?

## Recommendation

**Yes to 1**, in the order movie, sound with `/F`, assets — the entry's words already say it, and the
hand-off and its level exist. **For 2, the RIFF hand-off with the movie round, and Tier B still on
demand**: the wrapper keeps every sample and starts nothing in-process, and the demand has not come.

**What the tree does meanwhile**: the actions are refused by name and the windows print why; the
poster and the icons are drawn; the rows stay `out-of-scope`.
