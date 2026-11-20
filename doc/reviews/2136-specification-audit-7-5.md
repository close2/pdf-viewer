# Specification audit of ISO 32000-2 §7.5 and §7.3.10 — round 2136

The project's third specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits 2128 and
2131 ([`2131-specification-audit-7-6.md`](2131-specification-audit-7-6.md)). Baseline: main at
`5ab240a3`. Evidence: [`2136-specification-audit-7-5/`](2136-specification-audit-7-5/) —
`findings.json` (2038's schema) and `expected-failures.json` (one failing test per behaviour
finding that a test can reach). The same entries are in
`2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §7.5.1 to §7.5.8.4 (Tables 15 to 19), §7.3.10 and §7.3.8.2's
   `/Length` was read in `doc/md/ISO_32000-2_sponsored_EC3.md` (lines 1486 to 1600 and 2053 to
   2460, the figures' image lines left out).
2. Everything a reader owes was traced through `pdf_syntax`'s `xref.rs` (`read`,
   `read_from_startxref`, `read_classic_table`, `read_xref_stream`, `entry_location`,
   `walk_revisions`), `parser.rs`'s stream extent, `document/parsing.rs`'s indirect `/Length`,
   `document/objects.rs` (`get`, `load`, `resolve`) and `document/object_streams.rs`; everything a
   writer owes through `write.rs` (the incremental update) and `serialize.rs` (the whole file,
   object streams and both section forms).
3. The habits of 2128 and 2131 were the lens: which definition of an object wins, across updates
   and between a hybrid file's table and its stream; what an offset is counted from, on the way in
   and on the way out; what a compressed object's extent is; and what each record of a section
   costs a reader that has not been given a budget for it.
4. Each B finding a test can reach has a failing `*_spec_audit` test in
   `crates/pdf-syntax/tests/file_structure_spec_audit.rs`, built from bytes the test writes, with a
   control in the same test that passes on main.

What holds is not listed one by one. Among it: offsets read from the header's position for
`startxref`, `/Prev`, `/XRefStm` and every entry (ADR 2960); §7.5.6's newest copy, a free entry
recorded as a deletion an older section cannot undo, and a type outside 0 to 2 read as null; the
hybrid order section, then `/XRefStm`, then `/Prev`, the same in `read` and `walk_revisions`;
`/W`'s zero widths and defaults, `/Index`'s `[0 Size]`, fields wider than eight bytes clamped;
`/Prev` cycles and the 1024-section cap; a generation read from the object's own header, so a
reference at another generation (or a non-zero one to a compressed object) is null (§7.3.10);
reference chains followed to a stated depth of 64; an indirect `/Length` re-applied once the
document can resolve it, so data containing `endstream` keeps its stated extent; an object stream
reached only at the outermost level, so streams that contain themselves or each other end at
once, and `/Extends` never followed; strings in compressed objects never decrypted a second
time; new objects numbered past every stated number, never from the free list; the whole-file
writer's header, binary comment, object 0 at 65 535, `/ID`, object streams that hold no stream,
no bare reference and no encryption dictionary, and offsets refused by name past their field.

**One premise of the brief did not hold**: §7.5.7 does not say an object stream's dictionary is
unencrypted. It says "strings occurring anywhere in an object stream shall not be separately
encrypted"; the object stream's own dictionary is an ordinary indirect object's, encrypted under
§7.6.2. The dictionary §7.5.8.2 exempts is the cross-reference stream's (2131-B05, fixed).

## Counts

| kind | count |
|---|---|
| B (behaviour) | 5 |
| T (test) | 1 |
| D (documentation) | 2 |

## Findings

**2136-B01 — a compressed object runs into the next one.** §7.5.7 NOTE 7 (2020): "processing of
each object in an object stream starts at the specified byte offset in the decompressed stream
and ends prior to the byte offset of the next object or when the end of stream is encountered",
because the 2020 edition removed the white-space between members. `expand_object_stream` computes
that end and parses over the whole data, so `57` stored as object 10 at 0 and object 11 at 1 reads
as `57` and `7`. Test: `a_compressed_object_ends_where_the_next_one_begins_spec_audit`.

**2136-B02 — an update to a file with bytes before its header is read by scanning.** §7.5.2:
"byte offsets shall be calculated from the PERCENT SIGN (25h)". `write::incremental_update_extending`
states each new offset and the new `startxref` as a position in the whole file; the reader adds
the header's position, finds no section, and rebuilds. The freed object of the test comes back
as `(original)`, and the rebuilt document is refused by every later update
(`UpdateError::Recovered`), so the file can be saved once. Every save path in the tree uses this
writer. Test: `an_update_to_a_file_with_bytes_before_its_header_counts_from_the_header_spec_audit`.

**2136-B03 — free entries cost the reader memory apiece, past any budget.** CLAUDE.md principle
3 asks for budgets against xref bombs; §7.5.4 lets a conforming file state "one entry for each
object number from 0 to the maximum object number defined", and a cross-reference stream states
them in a few bytes. A conforming 526-byte file with two million free entries behind two
`FlateDecode` filters grows the process by about 108 MiB; the decode stops only at
`max_stream_len` (1 GiB, some 200 million entries), and every one of up to 1024 sections is held
until `fill`. Test: `free_entries_do_not_cost_the_reader_memory_apiece_spec_audit`.

**2136-B04 — the whole-file writer's index field overflows.** Table 18 type 2 field 3 is "[t]he
index of this object within the object stream"; `serialize` writes it in two bytes with
`unwrap_or(u16::MAX)`, and `ObjectStreams::Generate`'s `max_objects` is public and unbounded. In a
stream of 70 004 members the last is stated at index 65 535. This reader is unaffected (it finds a
member by number); a reader that follows the index is not. Test:
`a_compressed_entry_states_its_own_index_however_many_members_spec_audit`.

**2136-B05 — the update writer writes an offset its field cannot hold.** §7.5.4's entry is "a
10-digit number" in "exactly 20 bytes"; `write.rs` formats `{offset:010}` (eleven digits past
10^10) and writes a stream offset as `u32::try_from(offset).unwrap_or(u32::MAX)`. `serialize`
refuses both by name. No test: it needs a document past 4 GiB held whole.

**2136-T01** — no test pins that a hybrid section's own free entry wins over its own `/XRefStm`,
which is §7.5.8.4's order read literally; the existing test covers the previous section's.
**2136-D01** — `load`'s fallback `expanded.values().nth(index)` counts in object-number order,
not the stream order Table 18's index means (NOTE 6), and substitutes another member's value.
**2136-D02** — `incremental_update`'s doc says the form follows the *last* section and the
generation comes from the table; the form follows the merged trailer's `/Type`, and the table
holds no generation.

## Ledger rows a behaviour finding contradicts

Of the fifteen rows audited, 7.5.5 is `departed` (Table 15's `/Size` rule, ADR 2346) and fourteen
are `implemented`. The findings contradict **7.5.7** (B01, whose note credits NOTE 7 with the
parse by offset, and B04), **7.5.2** and **7.5.6** (B02), **7.5.4** (B02, B03, B05),
**7.5.8.3** (B02, B04, B05) and **7.5.8.2** (B03), and through them the aggregates 7.5 and
7.5.8. They are not moved here; the fix round decides.

## Suggested fix groups

| group | findings | files |
|---|---|---|
| A — a compressed object's extent | B01, D01 | `pdf-syntax/src/document/object_streams.rs` (`expand_object_stream`), `document/objects.rs` (`load`), ledger 7.5.7 |
| B — the update writer's offsets | B02, B05, D02 | `pdf-syntax/src/write.rs`; the header position exposed from `xref.rs` through `Document` |
| C — the cross-reference budget | B03 | `pdf-syntax/src/xref.rs` (`read_xref_stream`, `decode_direct`, `XrefTable::fill`, `read_from_startxref`), `document/object_streams.rs` (`object_stream_pairs`) |
| D — the whole-file writer's index | B04 | `pdf-syntax/src/serialize.rs` (`cross_reference_stream`, `ObjectStreams`) |
| E — the hybrid order | T01 | `pdf-syntax/tests/cross_references.rs` |

A, B and D touch different files and can run as one round each; C is the one that needs a design
(a run of free entries held as a range, or a named budget), and B's header position is a one-line
read C does not touch. The most consequential is B02, which undoes a deletion and leaves a file
that cannot be saved again; B03 is a budget the principles name; B01 misreads a conforming file.
