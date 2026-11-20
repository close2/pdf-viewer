# 3249 — An update counts its offsets from the header, and refuses one its field cannot hold

Session 2137. Status: **implemented.** Context: audit 2136 findings B02, B05 and D02, ISO 32000-2 §7.5.2, §7.5.4,
§7.5.6, §7.5.8.3 Table 18, ADR 2960.

## 1. The findings

§7.5.2: "byte offsets shall be calculated from the PERCENT SIGN (25h)", and NOTE 1 allows bytes before the header.
ADR 2960 taught the reader to count from the header; `write::incremental_update_extending`, which every save path
uses, still stated each new entry and the new `startxref` as a position in the whole file. On a file with bytes
before its header the reader added the header's position to them, found no section, and rebuilt by scanning: an
object the update freed came back, and the rebuilt document was refused by every later update
(`UpdateError::Recovered`), so the file could be saved once (B02). The same writer formatted a table offset as
`{offset:010}`, eleven digits past 10^10, and clipped a stream offset with `unwrap_or(u32::MAX)` (B05). Its doc said
the section form followed the *last* section and the generation came from the table; the code read the merged
trailer's `/Type`, and the table holds no generation (D02).

## 2. The decision

An offset has an origin on the way out as well as on the way in. `XrefTable` records the origin its chain was read
from (`offset_origin`), `Document::offset_origin` exposes it, and the update writer subtracts it from every offset
it states. `XrefTable` also records whether the section `startxref` names is a table or a stream
(`newest_section_is_classic`), and the update's form follows that section, which is what the doc always meant: a
table appended over a file of stream sections merges an older `/Type /XRef` into the trailer, and is still a
table. An offset past its field is refused by name, `UpdateError::OffsetTooLarge`, as `serialize` refuses it
(`SerializeError::OffsetTooLarge`); widening the stream's offset field instead was not chosen, because the table
form has no wider field to give and one rule for both forms is the clearer one.

## 3. Cost

Two words in `XrefTable`. A file of 10^10 bytes or 4 GiB held whole is refused where it was written wrong; no test
writes one, since the test would need the file in memory.
