# 2447 — An offset and a timespan are read for viability, and their subtype rule names the rendition

Session 1806, the clause slot of batch 121 (ADR 2370's ledger rule).

Status: accepted. §13.2.6.2 and §13.2.6.3 move from `out-of-scope` to `partial`, and §13.2.6 above
them becomes an aggregate. The test is
`crates/pdf-model/tests/media_viability.rs::an_offset_names_its_clips_start_only_as_zero_seconds_or_frame_zero`,
beside ADR 2431's. No code changes: `media.rs`'s judgement is another slot's this batch.

## Context

Both rows were `out-of-scope` on clause 13's exclusion. Their notes quoted "shall specify an offset
into a media object" and "A timespan dictionary shall specify a length of time" as the player's
business. Round 1798 (ADR 2431) then made `pdf_model::media` read both tables to decide whether a
section's `/B` names its clip's start. The seventh sweep named both rows by that file. The brief asked
whether `CLAUDE.md`'s clause 13 entry admits a move.

## Decision

**The entry divides each clause in two, and the rows are `partial`.** The entry puts §13.2.2's
viability, computed and reported in the clause's own terms, in scope. It excludes the decoding and
presenting of audio and video. Reading a dictionary's subtype and number decodes nothing, so each
sentence that bears on viability is in scope. Seeking to an offset, or timing a clip to a span, is the
player's and stays excluded. The ADR 2428 decision that reopens the player reopens that half too.

**Executed, and named in each row**: Table 296's `/S`, Table 297's timespan, Table 298's "Frame
numbers begin at 0; negative frame numbers are out of range", and Table 300's `/S /S` with a `/V` read
as a number ("Non-integral values shall be allowed"). `is_start` reads all four for a `/B`.

**Owed, and the reading a later round builds on.** Table 296 and Table 300 each say "The rendition is
non-viable if the interactive PDF processor does not recognise the value of this entry". §13.2.2's
best-effort rule is about options: "if they cannot be honoured, the containing object is still
considered viable". These two sentences are about a value not recognised, and they name the
rendition. Nothing in them limits them to `MH`. So an unrecognised `/S` makes the rendition
non-viable wherever the dictionary sits:

- in a section's `MH` or `BE` `/B` or `/E`;
- for Table 300, also in a media duration's `/T` under the play parameters' `BE`.

Table 292's own subtype rule names only "[t]he media duration dictionary", and is §13.2.5's to read.
Today a `BE` offset of that kind is said as unhonoured and the rendition is chosen. In `MH` it is
non-viable, but for Table 289's reason, which is not the clause's own terms. **Not owed by this
reading**: an absent `/S`. The sentence is about a value the processor does not recognise, so an
absent `/S` stays a best-effort offset not honoured.

The second item owed is what a later offset or a marker names inside the clip. That needs the
clip's container, as §13.2.4.3's two items do.

## Consequences

- Both rows say what moves them: the judgement in `media.rs`, extended. That is "a normal round
  extending the existing code", `doc/todo/65`'s fourth bucket, where both now sit. §13.2.6 is in
  its aggregate list.
- `exclusion = "clause-13-multimedia"` leaves both rows, because only an `out-of-scope` row may
  carry one. The excluded half is argued in each note instead.
- The new test was checked against a planted defect, a frame offset of `<= 0` read as a start. It
  failed on `/F -1`.
