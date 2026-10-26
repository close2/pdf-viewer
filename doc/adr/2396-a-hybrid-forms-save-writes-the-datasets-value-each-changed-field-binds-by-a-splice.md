# 2396 — A hybrid form's save writes the `datasets` value each changed field binds, by a splice

Session 1781, a clause round in batch 118. Status: implemented. ADR 2397 is unused.

## Context

§K.2's first writer rule binds any writer "modifying a PDF file with an XFA resource": "The XFA
field values shall be consistent with the corresponding V entries of the PDF field objects."
`ViewState::save` wrote `/V` and left the `datasets` packet as the producer wrote it (ADR 2380, RFC
0011 section 3.1), so every filled, saved hybrid form broke the rule. RFC 0011 section 13 item 2 made
reading the resource and keeping the packet the static half's second round. XFA 3.3 is cited by
chapter and printed page and paraphrased (RFC 0011's preamble).

## Decision

1. **The resource is read in both of §K.2's forms** (`pdf_model::xfa::Resource`): the array by its
   `template` and `datasets` packet names, the single stream as one XDP document. Each packet is
   decoded under a 32 MiB bound and parsed by the workspace's `xmlparser` into a tree that keeps each
   element's byte ranges, under depth and element budgets, with end tags checked against start tags.
2. **A PDF field is found in the template by its name.** XFA 3.3 chapter 2, *Field Names* (pages
   72–73), makes an XFAF form's field name a SOM expression; chapter 3's class references (`#subform[0]`,
   page 96) and transparent nodes (pages 94–95) are the grammar the hybrids use.
3. **The data value is found by chapter 4's direct match**: the root subform binds the record, the
   first data group under `xfa:data`, whatever the names (page 182); each named, normally bound
   subform is one step; same-named siblings pair by index (page 195); nameless subforms and fields,
   `scope="none"` subforms and areas are passed through (pages 193–194). `bind match="global"` takes
   the direct match first and then a value of its name outside the record (pages 246–247);
   `match="dataRef"` follows `$record.`, `$data.` and `$.` paths of plain names; `match="none"`
   writes nothing, because the template binds that field to nothing.
4. **A subform stating `bind match="none"` is passed through, as a reading.** Page 176 says such a node
   binds no data and says nothing of its children; the only examples (pages 358–359) keep binding the
   children by name in the wrapper's parent. `bug1815476.pdf`, the one hybrid that states it, has its
   values directly under the record. That file is evidence that the reading agrees with a producer.
   It is not the reading's source.
5. **The value is the field's in XFA's terms**: a text field's text; a check button's first `items`
   value when on and its second when off (page 759); a named exclusion group's on value of the button
   that is on, matched to Table 230's `/Opt` export value of the widget whose state is on; a choice's
   export value, and a multiple-selection list's data group of `value` elements (page 198).
6. **The packet is spliced, not re-serialised.** Changed values get new content; a value the data
   lacks is appended to its group, as chapter 4's re-normalisation does (page 210); every other byte
   is the producer's. The stream is replaced by §7.5.6's update with its filters removed. A field
   counts as changed where its inherited `/V` reads differently through the update than through the
   file, so typing, a reset, an import and a script are all kept, and a save that changes no field
   writes no packet.
7. **What it cannot keep is said, not guessed** (trap 5). `Written::xfa_unkept` names each such field
   and `viewer-core` reports it. The cases: a numeric, date or other non-text field, whose data is
   chapter 3's canonical form (RFC 0011 step 3's); rich text; a button of a named group; a field on a
   master page; a binding this does not follow; a dynamic form (Table 29's flag, or page 333's test,
   which waits on Q639); a resource without a `datasets` packet.
8. **The `form` packet is left as written.** XFA 3.3's XDP chapter lists no `form` packet and states
   nothing about its `checksum`, so nothing here rests on it.

## Consequences

- `crates/pdf-model/tests/xfa_datasets.rs` fills, saves and re-reads the corpora's four hybrids:
  text with a carriage return and markup characters, check buttons, radio groups, single and
  multiple choices, and a global choice. No field is left unkept, every filled value reads back, and
  every unfilled value reads back unchanged. With the write removed, four of its five tests fail.
- Annex K's rows stay `out-of-scope` (RFC 0011 section 11); K.2's note says what is read and kept.
- Next is RFC 0011 step 3, formatting. That step is also what lets a numeric or date field's data be
  written.
