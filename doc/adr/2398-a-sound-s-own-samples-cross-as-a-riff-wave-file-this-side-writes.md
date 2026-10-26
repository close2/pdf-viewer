# 2398 — A sound's own samples cross as a RIFF WAVE file this side writes

Session 1782, slot 4 of batch 118. Status: implemented. Context: `CLAUDE.md` principle 5's clause 13
entry; `doc/questions/A631`; ADRs 1961, 2382; ISO 32000-2 §13.3, Table 305; RFC 2361; Microsoft's and
IBM's *Multimedia Programming Interface and Data Specifications 1.0* (1991).

## Decision

§13.3's "[i]f no F entry is present, the sound object itself shall contain the sample data and all other
information needed to define the sound" names samples no player can be handed as they stand. A631 settles
the build: they become a RIFF WAVE file, the form the clause's own NOTE lists among the self-describing
ones, and cross through the hand-off a sound's `/F` already uses (ADR 2382). Nothing plays in-process.

- **The judgement reads, the hand-off writes.** `pdf_model::media::sound` reads Table 305 into
  `SampleFormat` and returns `Data::Samples` with the stream undecoded; `viewer_core`'s hand-off decodes
  the stream through Table 5's filters only once the level has let the clip through, then calls the pure
  `pdf_model::media::wave`. `Event::Media` and `viewer_core::Media` do not change shape, so no window,
  wire or C ABI changes; the file is `Media::Embedded { name: "sound.wav", .. }`.
- **The layout.** PCM samples (`Raw`, `Signed`, 1 to 32 bits) are unpacked from §13.3's most-significant-
  first packing, set in the high bits of the smallest whole number of bytes, written least significant
  byte first, and re-biased where the two signedness rules differ: `Signed` at eight bits or fewer becomes
  unsigned, `Raw` above eight becomes two's complement. µ-law and A-law codes cross unchanged under RFC
  2361's tags 7 and 6, with the extra-size field and the `fact` chunk a non-PCM file carries. The content
  type is RFC 2361's `audio/vnd.wave; codec=<tag>`.
- **What is said, never dropped** (trap 5). A `/CO` — Table 305: "No standard values have been defined for
  the CO and CP entries" — hands nothing over and says so; so do an `/E` the table does not name, µ-law or
  A-law codes that are not eight bits, and a `/C`, `/B` or `/R` a WAVE header cannot state. A fractional
  `/R` is rounded and said; whole samples that complete no frame are dropped and said.
- **The bound** is the document's `Limits::max_stream_len`, applied to the file written, because one-bit
  samples unpack to eight times the bytes read; no second number is chosen (trap 38).

**The premise that did not hold**: `doc/md/`'s Table 305 drops the `ALaw` row and `Default value: Raw.`
that ISO 32000-2's printed table carries; the code follows the printed table and says so in a comment.

## Cost

One `Data` variant, matched in `pdf-model` and `viewer-core`. §13.3 moves `partial` → `implemented`.
