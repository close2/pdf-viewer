# 2472 — A best-effort duration's unrecognised timespan makes the rendition non-viable, in Table 300's words

Session 1819, the clause slot of batch 122 on §13.2.5's media duration and §13.2.6.3's timespan.

Status: implemented — `crates/pdf-model/src/media.rs` (`unrecognised_duration`, `read_timespan`), tested by
`crates/pdf-model/tests/media_viability.rs::an_unrecognised_timespan_in_a_best_effort_duration_makes_the_rendition_non_viable`.

## Context

ADR 2447 read Tables 296 and 300 — "The rendition is non-viable if the interactive PDF processor does
not recognise the value of this entry" — as naming the rendition, and executed it for a section's `/B`
and `/E` in `BE` as in `MH`, because a sentence that names the rendition is not set aside by the
dictionary that holds the entry. ADR 2452 left one use of the same sentence: Table 292's `/T`, the
explicit duration of a media duration dictionary `/D` in the play parameters' `BE`. Table 291 asks
that dictionary's entries to be honoured only in a best-effort sense, so the duration, whatever was
wrong with it, left the rendition chosen.

## Decision

1. **A `BE` duration whose `/S` is `T` and whose `/T` timespan states an unrecognised `/S` makes the
   rendition non-viable**, with the reason said in Table 300's own sentence. Table 292 reaches `/T`
   only "if the value of S is T"; for `I` and `F` it is "otherwise ignored", so it is not read.
2. **Every other fault in a `BE` duration is set aside, as Table 291 allows.** A duration whose own
   `/S` is not recognised is, in Table 292's words, a dictionary that "is non-viable": the sentence
   names the dictionary and not the rendition, and a best-effort dictionary may be ignored. A
   timespan with no `/S`, a negative `/V` ("Negative values shall not be used") or no `/V` states no
   value the processor fails to recognise; it is an entry no player can follow, which a best
   effort ignores. The two sentences differ in what they name, and the code follows each in its own
   words rather than reading the stricter one into both (trap 5).
3. **One reading of a timespan.** `read_timespan` is Table 300 once; a media offset's `/T`
   (Table 297) and a media duration's `/T` both call it, so one dictionary gets one answer.
4. **An `MH` duration is untouched**: every `MH` entry of Table 291 already makes the rendition
   non-viable, because a player handed only the file cannot be told how to play it (ADR 1960), and a
   second reason would say the same thing twice.

## Consequences

- §13.2.6.3 moves `partial` → `implemented`: the sentence its note owed is executed, and what is left
  of the clause (timing the clip to the span) is the player's, excluded by `CLAUDE.md`'s clause 13
  entry as before. §13.2.5's note names the new reading.
- No corpus document holds a media duration with a timespan (ADR 2452's census found no media clip
  section; the rule is the standard's and the population is not evidence for it, trap 8).
