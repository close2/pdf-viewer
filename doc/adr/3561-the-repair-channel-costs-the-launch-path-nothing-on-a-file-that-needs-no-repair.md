# 3561 — The repair channel costs the launch path nothing on a file that needs no repair

Session 2215. Status: accepted. Context: `CLAUDE.md` principle 2 ("Perf gates run at every merge …
A regression fails the gate"); ADRs 3477, 3505, 3541; `doc/checks/launch-path.toml`.

## The regression

From e2008ed8 the launch gate's counted figures stood outside their bands: `open_kinstructions`
on all six documents and `frame_kinstructions` on EC3 and `opt_demo`. The counts are independent
of load, so the commit that held the bands and the tree were each built under `[profile.release]`
and every document's open and first frame counted under callgrind, the profiles diffed by
function. Every function that grew was `pdf-syntax`'s, and the one commit in the range that
touched the lexer, the parser and the cross-reference reader is `ac03b9082` (round 2201, the
repair channel, ADR 3505). The content reader's per-token `keep_repairs`, the suspect named, was
not the cause: one length test a token, it moved EC3's first frame by 24 thousand of 127 million.
Two causes were:

1. **The parser's look-ahead cloned the lexer.** `integer_or_reference` asks at every integer
   whether it begins `N G R`, and `parse_dictionary_or_stream` whether `stream` follows, each on a
   clone of the lexer. ADR 3505 gave the lexer a `Vec<Repair>`; the clone then copied and the
   assignment dropped a vector at every integer and every dictionary.
   `Parser::parse_object_from` and `memcpy` were most of every document's growth.
2. **The cross-reference stream's record loop.** Each record was tested for
   `Entry::Unrepresentable` and the section's repair list was live across the loop; the loop
   turns 112 thousand times on EC3, and its disassembly showed seven instructions more a record,
   the test's and values spilled to the stack.

## Decision

1. **A mark, not a clone.** `Lexer::mark` copies the cursor, what was examined, the hexadecimal
   counts, the salvage, the wide integer and the repair log's length; `Lexer::restore` puts them
   back and truncates the log. The look-ahead runs on the lexer itself and is taken back unless it
   is a reference or a stream: the state a clone made at the mark would have had, repairs noted in
   the look-ahead included.
2. **The rare record found apart.** A type 1 offset or type 2 field no `Location` holds can only
   come from a `/W` field wider than four bytes; `RecordLayout::can_overflow_a_location` asks that
   once a stream, and only then does `unrepresentable_records` pass over the records, in the
   loop's own order, and log `syntax/xref-stream-record` once. `entry_location` reads such a record
   as free, as before. The record loop is `read_records`, a function of its own, so that only the
   entries are live in it, and `unrepresentable_records` is cold and out of line. Two other shapes
   were built and counted on EC3's open and are not taken: a flag `entry_location` sets (1.26
   million more, its register taken from the loop), and `read_records` never inlined (2.0 million
   more, its entries crossing the call in memory).

## The figures (thousands of instructions in the program's own object; bands from the toml)

| figure | band | e2008ed8 | before | after |
|---|---|---|---|---|
| AN001-BPC open | 2179 .. 2224 | 2213.3 | 2248.9 | 2186.5 |
| WTPDF open | 9997 .. 10200 | 10100.2 | 10320.2 | 10094.1 |
| EC3 open | 53527 .. 54609 | 54059.3 | 55150.6 | 54418.1 |
| EC3 frame | 124448 .. 126963 | 125794 | 127492 | 126137 |
| bug1815476 open | 1744 .. 1780 | 1766.3 | 1789.8 | 1763.6 |
| xfa_filled open | 279 .. 286 | 284.9 | 287.9 | 284.4 |
| opt_demo open | 400 .. 409 | 407.9 | 412.6 | 405.2 |
| opt_demo frame | 3118 .. 3183 | 3175.6 | 3184.9 | 3175.3 |

No band was moved.

## What the repair channel keeps

Every repair is logged where it was: the look-ahead's repairs survive a committed reference and go
with a rewound one, as they did on the clone; an out-of-range reference is still noted at its
integer; an unrepresentable record is still logged, now once a stream. `repair_classes`, the
`silent_repairs_spec_audit` tests and `pdf-syntax/tests/xref_stream_record_repair.rs` (a five-byte
`/W` naming an object stream past `u32`, and one inside it) hold it.
