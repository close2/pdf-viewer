# 2431 — A must-honour begin at the clip's start is honoured by the hand-off

Session 1798, the clause slot of batch 120 (ADR 2370's ledger rule).

Status: implemented — `crates/pdf-model/src/media.rs` (`is_start`, `Walk::begins_at_start`), tested
by `crates/pdf-model/tests/media_viability.rs::a_must_honour_begin_at_the_clips_start_is_honoured_by_the_hand_off`.

## Context

§13.2.4.3 was `partial` with three owed items, one of them an `MH` `/B` at the clip's start judged
non-viable although the hand-off — the one player this program has, told nothing but the file (ADR
1960) — starts the file there (ADR 2392). The brief asked whether `CLAUDE.md`'s clause 13 entry
admits reading §13.2.6.2's offset for that. It does: the entry puts "§13.2.2's viability computed and
reported in the clause's own terms" in scope and excludes "decoding or presenting audio and video".
Reading a dictionary's subtype and number decodes nothing.

## Decision

A `/B`, in `MH` or `BE`, is honoured where it names the start of its next-level clip and that clip
begins where the file does:

- **The offset names a start** (`is_start`) when it is a Table 297 time offset whose §13.2.6.3
  timespan has `/S /S` and `/V` zero, or a Table 298 frame offset of frame 0, "[f]rame numbers begin
  at 0". A marker names a place only the container's marker table locates; an unrecognised Table 296
  or Table 300 `/S` is non-viable by those tables' own sentences; a negative or later time, or a later
  frame, is a seek.
- **The next-level clip begins at the file's start** (`Walk::begins_at_start`) when it is media clip
  data, or a section each of whose `/B`s names a start of a clip that begins there in turn: a time or
  frame `/B` is "relative to the start of the next-level media clip", so a zero over a section that
  begins at frame 20 is frame 20. A best-effort `/B` deeper in the chain the hand-off does not honour
  therefore still moves the start, and the section above it stays non-viable; the chain is bounded by
  `MAX_DEPTH`, as `Walk::clip` is.

`/E` is unchanged: whether an end is "beyond the end of the next-level media clip" needs the clip's
duration or frame count, which only its media container states.

## Consequences

- §13.2.4.3 stays `partial`, now on two items — the `/E` past the clip's end and the marker the clip
  lacks — and both need a reader of each content type's container metadata, which no round has built.
- §13.2.6.2's and §13.2.6.3's rows, `out-of-scope`, are now executed in part by viability — Table
  296's and Table 300's subtype rules, Table 297's "[n]egative timespans are prohibited in this
  context" and Table 298's frame 0 for a `/B` — and the seventh sweep names both by `media.rs`. They
  are outside this slot's rows and are left for the slot that owns them, with this reading.
