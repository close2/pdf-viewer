# 2392 — A media player info dictionary is judged before its array is read, and an offset over a still image is ignored

Session 1779, the clause slot of batch 118 (ADR 2370's ledger rule). ADR 2393 is unused.

Status: implemented — `crates/pdf-model/src/media.rs`, tested by five fixtures in
`crates/pdf-model/tests/media_viability.rs`, one per subclause the rows cite.

## Context

ADR 2376 read §13.2's family into the ledger and found two verdicts of `pdf_model::media` contrary
to the clause. `Walk::clip` made a media clip section non-viable for an `MH` `/B` or `/E` over any
clip, where §13.2.4.3 says "For media types where an offset makes no sense (such as JPEG images), B
and E shall be ignored, with no effect on viability". And `media::players` read `/MU`'s emptiness off
the array as written, where §13.2.7.2.1 says a non-viable media player info dictionary "shall be
treated as if it were not present in its original array", so an `/MU` of such entries refused a clip
the clause makes viable. Four `partial` leaves waited on the second: §13.2.7.2.1, §13.2.7.3,
§13.2.7.4.1 and §13.2.7.4.4.

## Decision

1. **A section's next level is judged first**, because the data's Table 285 `/CT` decides whether
   the section's offsets count; the section's own reasons are still said before the next level's.
2. **"Media types where an offset makes no sense" are the registered still-image types whose
   format holds one picture and no timeline**: `image/jpeg`, `image/png`, `image/bmp` and
   `image/jp2`, matched without parameters and without regard to case (RFC 2045 section 5.1). The
   clause gives JPEG as its example and states no list, so the list is a choice. A type whose format
   can animate — `image/gif`, `image/webp`, `image/svg+xml` — is left out, and so is any type not
   named; there the offsets keep their effect, which is the verdict the tree gave before. Adding a
   type is a one-line change with its reason.
3. **Each array entry's own viability is judged before an array's emptiness is read**
   (`media::player_info_set_aside`), in this order: an entry that is no dictionary; an `MH` holding
   any entry, since Table 302 defines none and §13.2.2 makes an unrecognised `MH` entry non-viable;
   a missing `/PID`; a software identifier holding a key Table 303 does not define (§13.2.7.4.1's
   partially recognised, which §13.2.7.2.1 sets aside); a missing `/U`; a negative number in `/L`
   or `/H` (§13.2.7.4.4).
4. **A required entry's absence is read as non-viability**, for `/PID` and `/U` alike — the reading
   this module already gives Table 288's `/D`, and §13.2.2 ties a requirement to viability ("If an
   entry is required, the requirement is met if the entry is present in either the MH dictionary or
   the BE dictionary"). A `/U` whose scheme is not recognised is different: §13.2.7.4.3 calls that
   software "a non-match", and the entry stays.
5. **An entry set aside is said** in `Considered::set_aside` and the report's notes, because a
   removal can be what turns an `/MU` empty and a refusal into a hand-off.
6. **§13.2.7.2.1's other half decides nothing here**: a dictionary with the same software identifier
   "logically considered present in NU" binds condition (b), which needs the player found by that
   identifier, and no identifier names the hand-off's player (ADR 2376).

## Consequences

§13.2.7.2.1, §13.2.7.3, §13.2.7.4.1 and §13.2.7.4.4 move `partial` → `implemented`, and §13.2.7,
§13.2.7.2 and §13.2.7.4 with them. §13.2.4.3 stays `partial`, and so do §13.2.4 and §13.2 above it,
on three verdicts still contrary to the clause: an `MH` `/E` past the next-level clip's end, and a
marker the clip lacks, each "no effect on viability", and an `MH` `/B` at the clip's start, which
the hand-off honours by starting there. The first two need the clip's duration, frame count and
marker table, which only the media container holds — a reader of each content type's container,
which this tree does not have. The third needs §13.2.6.2's offset value, a row that rests on clause
13's entry; reading it for that one comparison is the next step on this row.
