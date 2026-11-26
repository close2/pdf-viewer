# 3777 — The tenth pass empties `HELD` but for two rounds' files, and makes zero the ratchet

Session 2269, the silent-repair campaign's census burn-down, tenth pass. Status: **accepted**.
Context: ADR 3477, ADR 3521, ADR 3574, ADR 3757, ADR 3765; ISO 32000-2 §7.8.2, §8.10.2, §9.6.5.1,
§9.8.1, §9.9.1, §9.10.3, §12.5.2, §12.6.4, §12.7, §12.11.1, §13.2.3, §14.8.5.

## Context

ADR 3765 left 74 sites held in 59 files. Seven of those files (`content/marked.rs`,
`content/transparency*`, `content/pattern*`, nine sites) belong to two rounds running beside this
one. The other 65 sites in 52 files were each read against the text its module, or the enclosing
function's own documentation, cites.

## Decision

- **New classes**, each outside content, mapped to section 5.1 alone (`base`), so the whole-file
  rewrite carries the entry as stated and the output's own check refuses by the class's name:
  - `model/requirement-entry-unusable` (§12.11.1): Table 273's `/Penalty` past 0 to 100, read as
    the nearer end, or of another type, read as its default 100. Logged at the requirement.
  - `model/media-criterion-unusable` (§13.2.3): Table 279's `/L` that is no array, or an element
    that is no text string, read as absent. An empty array is the clause's own "present and
    non-empty" condition, and is not logged.
  - `font/opentype-cff-unreadable` (§9.9.1): a `Type1` font's OpenType program whose `CFF ` table
    does not read for its charset is mapped through its `cmap`, the TrueType route, where §9.9.1
    gives it §9.6.5.2's. Logged at the descriptor.
- **On existing classes**, each reading unchanged unless said:
  - `font/to-unicode-damaged`: a destination holding an unpaired surrogate was decoded with
    U+FFFD in its place. §9.10.3 has the map's mappings "expressed in UTF-16BE encoding", so such a
    destination now maps nothing and marks the map damaged, as an odd-length one; the code falls
    to §9.10.2's next method. A `bfrange` array's or sequence's destination that does not read now
    marks the map too (before, only a `bfchar`'s did).
  - `font/descriptor-style-unusable`: Table 122's `/Style` that is no dictionary, or a `/Panose`
    that is no twelve-byte string.
  - `model/field-entry-unusable`: an `/Opt` element of neither of Table 234's shapes; a negative
    `/MaxLen` where an edit or a script holds a value to it; a non-integer `/Ff` where the view
    asks whether a button toggles.
  - `model/action-entry-unusable`: Table 212's `/Volume` past its range or of another type, and a
    `/Repeat`, `/Synchronous` or `/Mix` of another type. These were said only in the action's notes.
  - `model/structure-attribute-unusable`: a PrintField `/Checked`, `/TextDecorationType` or
    `/TextPosition` that is none of its table's names, read as the default.
  - `model/annotation-entry-unusable`: a `/Rect` absent or not four numbers where a script reads
    an annotation's rectangle (Table 166 makes it required).
  - `model/form-resources-not-a-dictionary`: an annotation appearance stream's `/Resources` of
    another type, logged as a form XObject's already was; the absent entry still inherits the
    page's, the choice ADR 0255 argues.
- **Not repairs**, each said beside the code: the catalog declines; slices and counts of this
  reader's own; declines whose refusal is named where a decode, a draw or the page meets it
  (`decoded_stream_data`, `NestedContent::of`, a `CMap` that cannot delimit codes); an `sh` or `Tf`
  short of an operand, logged by the loop or the content survey under §7.8.2's classes; a field
  with no value read by a script as the empty text; an XFA `xdpContent` omitted; an ASF description
  shown with U+FFFD; an `sfnt` already opened before its `CFF ` table is asked for.

## A ratchet at zero

When the two other rounds' files fall, `HELD` is empty. The census should then forbid a new held
site outright: the test's bound per file becomes zero for every file, and `HELD` is deleted rather
than kept empty. Kept, it invites the next round to add a line to it, which is the silent default
the campaign exists to end; deleted, a new default must arrive with its class or its marker, or the
census fails naming the line. What remains possible is a marker written without a reading — the
census's other direction of lying — and that is reviewed, not counted, as before.

## Consequences

- `HELD` falls from 74 to 9 sites; the 52 files read here hold none.
- `tounicode.rs` is on the per-font path; its change is one `collect` into a `Result`, measured by
  instructions in the record.
- **The archive census moves, and the movement is this log.** Identity `/ToUnicode` maps that
  state `<d800> <d8ff> <d800>` and its three neighbours: ranges whose destinations
  are surrogate code units, mapping codes no string of those files shows. Read before as U+FFFD and
  its successors without a word, they now mark the map damaged, and `font/to-unicode-damaged` is
  section 5.1's alone, refused by the output's own check (ADR 3570 point 5). Eight pdf.js documents
  and one of the corpora move from PDF/A-2b to PDF/A-4 under "either part", and one,
  `issue18099_reduced.pdf`, is refused at both as if printed. The remedy is ADR 3570's unbuilt
  route — write the mappings that read as a new `CMap`, which here loses only mappings of no
  character — and it is the first thing left.
