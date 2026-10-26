# 2382 — A movie's file and a sound's file cross through the rendition's hand-off

Session 1774, slot 4 of batch 117. Status: accepted, built. Context: `CLAUDE.md` principle 5's clause
13 entry; `doc/questions/A254`, `A631`; ADRs 1946, 1960, 1961, 1548; ISO 32000-2 §12.5.6.16,
§12.5.6.17, §12.6.4.9, §12.6.4.10, §13.3, §13.4. ADR 2383 is unused.

## Decision

The owner's A631 adopts Q631's first recommendation: the media file of a movie, then of a sound, is
handed to this machine's own player through the hand-off a rendition's clip already uses. Each of the
four clauses opens by handing itself to 13.2's framework, and the clause 13 entry admits the media
file handed over and excludes the playing, so nothing new is decided about scope — what is built is
three more sources for the one event.

- **One shape.** `pdf_model::media::movie` reads Table 189's `/Movie` and Table 306's `/F`;
  `pdf_model::media::sound` reads Table 212's or Table 188's `/Sound` and §13.3's Table 5 `/F`. Both
  return the `Clip` a viable rendition returns, in either file-specification form (Table 306 and
  Table 5 say "file specification"; the exclusion of simple ones is §13.2.4.2's own sentence), with
  Table 44's `/Subtype` as the type of an embedded file because Table 306 leaves the format
  unspecified. `Event::Media`'s shape does not change, so no window, wire or C ABI changes; the one
  sentence every window says, `viewer_host::handed_media`, cites §13.3 and §13.4 beside §13.2.4.2.
- **`Clip::temporary_file` is an `Option`.** Table 286 is a property of §13.2.4.2's clip data and
  no movie or sound dictionary states one, so for them the document asserts nothing against
  `Operation::HandOverMedia` and every level lets the file through. Reading Table 22's bit 5 into
  it instead would invent a restriction no clause states.
- **What the player is not told is said.** Table 307's activation entries (from the annotation's
  `/A` and, overriding them, the movie action's own, per §12.6.4.10), a non-zero Table 306
  `/Rotate`, and Table 212's `/Volume`, `/Synchronous`, `/Repeat` and `/Mix` are each a sentence
  (trap 5), as a clip's `BE` entries are.
- **Table 189's `/A false` withholds the annotation's own activation only**; a movie action's Play
  is the stronger statement by §12.6.4.10's override sentence. A `/T` is looked for on the page a
  jump earlier in the chain reached, else the page shown — the clause's associated page.
- **Stop, Pause and Resume act on nothing playing**, as a rendition's do, and say so.
- **Activation** is §12.5.1's "by playing a sound or a movie": a click (after Table 197's `/U`)
  and `Command::Activate` both reach `interact::exhibited`.

## Owed, and what each needs

- **A sound holding its own samples** (§13.3 stays `partial`): a RIFF wrapper written on this side
  for the hand-off — Table 305's `/R`, `/C`, `/B` into a `fmt ` chunk, µ-law as format 7 unchanged,
  8-bit `Signed` re-biased to unsigned, 16-bit big-endian swapped, a `/CO` with no standard value
  said — and then `Media::Embedded` with the wrapped bytes; nothing else moves.
- **Rich-media assets**: §13.7.2.3.2's `RichMediaContent` `/Assets` name tree walked by
  `attachment::attachments` (its file specifications are §7.11.4's), so they list and save under the
  existing `EmbeddedDocuments` level and are never handed to a player (A631).

## Cost

Two `Action` and two `Request` variants, matched in `pdf-model` and `viewer-core` only. The brief
named §12.5.6.25 for the movie annotation; that row is §13.6.2/§13.7.2's pointer and was untouched,
and §12.5.6.17 is the movie annotation's.
