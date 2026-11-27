# 3781 — A damaged `/ToUnicode` is written again from the mappings that read, at `font-maps-as-read`

Session 2270. Status: **accepted** and built; builds ADR 3570 point 5 for `to-unicode-damaged`.
Context: ADR 3565 (the classes), ADR 3570 (the row and the loss), ADR 3777 (an unpaired surrogate
read as no mapping); ISO 32000-2 §9.10.3, §9.7.6.2, Table 118; ISO 19005-2 and ISO 19005-4 section
5.1, ISO 19005-2 section 6.2.11.7.2.

## Context

Round 2269 stopped reading an unpaired surrogate in a `/ToUnicode` destination as U+FFFD: the entry
maps nothing and the map is logged `font/to-unicode-damaged`. That class was section 5.1's alone,
so the output's own check refused it by name, and twenty archive outputs that had converted were
refused (record 2269). §9.10.3 states the map's grammar — destinations "expressed in UTF-16BE
encoding", `beginbfchar` and `beginbfrange` sections, codespace ranges "consistent with the
encoding that the font uses" — and cites Adobe Technical Note #5014 for the rest of the syntax
(the brief named #5411, the tutorial ISO 32000-1's bibliography lists; §9.10.3 cites #5014).

## Decision

1. **The class joins `fonts/font-maps-read-as-written`** (`pdf_archive::repairs`), so the
   converter's existing remedy for that row answers it.
2. **The map is written from the reading, not from the producer's text**
   (`archive::font_maps::to_unicode`): one `bfchar` entry per code `pdf_font` holds singly, one
   `bfrange` entry per span, each code in the width the producer wrote it in, in the order the
   reader consults them; the producer's codespace ranges that read are carried, else each width
   the codes use is given its whole range; a `/UseCMap` base is folded in and the entry dropped. It
   is written at the producer's stream object, so every font naming the stream reads the new map.
   To make this possible `pdf_font::tounicode::Mapping` carries each statement's code width, and
   `ToUnicode` names each entry it read past (`unread`, `unread_count`, the first 64 kept).
3. **Proved before written**: the new program, read by the same reader, is neither damaged nor cut
   by a bound, and answers every code either map states — by value alone and at each of the four
   widths — exactly as the producer's map answered it: the same text for every code the old one
   read, and nothing for the rest. A code neither map states is answered by neither, so the proof
   covers the code space, under a budget of 2^20 codes. A map cut by one of this reader's bounds is
   refused by name (`NOT_WRITTEN`): the cut mappings are not in the reading, and writing the
   reading would state the producer's codes mean nothing.
4. **The loss is `font-maps-as-read`, not a new name.** ADR 3570's argument is that the loss is
   the producer's words at places no reader reads by ISO 32000-2 and a reader with a convention of
   its own may. A destination that is not UTF-16BE is exactly that — the common convention reads an
   unpaired surrogate as U+FFFD, which is what this reader did before ADR 3777 — and so is a span
   whose high code is below its low, and an unclosed section's end. Both shipped profiles already
   authorise the loss as `discard`; no profile changes. The report's restatement line names each
   dropped entry as the producer wrote it.

## Consequences

- The twenty documents round 2269's logging refused convert again where nothing else stands in
  their way; the census in record 2270 names each.
- Text extraction from the output reads what it read from the source: nothing is gained or lost.
- A damaged encoding `CMap` is not this route (ADR 3782).
