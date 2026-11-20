# 3257 — Free cross-reference entries are runs, and in-use entries have a named budget

Session 2139. Status: **implemented.** Context: audit 2136 finding B03, CLAUDE.md principle 3, ISO 32000-2 §7.5.4,
§7.5.8.2 Table 17 (`/Index`, `/W`), §7.5.8.3 Table 18, ISO 32000-1 Table C.1.

## 1. The finding, measured

§7.5.4 lets a file state "one entry for each object number from 0 to the maximum object number defined in the PDF
file", and `/Index` defaults to `[0 Size]`, so a conforming cross-reference stream states millions of free entries
in a few hundred bytes. The reader held every record as a `(u32, Option<Location>)` in its section, every section's
records until `XrefTable::fill`, and every number in a `BTreeMap`. Measured in the audit's own test (nextest, one
process, VmHWM): the 526-byte file of 2 097 152 free entries grew the process by **110 568 KiB**. Nothing bounded the
in-use side either: `Vec::with_capacity(data.len() / row)` asked for 24 bytes per record of a stream that may decode
to `max_stream_len`, 1 GiB, which at one-byte rows is 24 GiB of address space against the worker's 4 GiB.

Real files, release build of `callgrind_open`, peak RSS of the process: ISO 32000-2 (110 219 + 2 050 records, 9 927
free in 881 runs) 11 MiB; pdf.js `issue16091.pdf` 11 MiB; pdf.js `bug1978317.pdf` (65 564 records) 136 MiB, of which
the table is a few, the rest being the document's other work. The largest real table this tree reads is ISO
32000-2's.

## 2. The decision

**Free entries are runs.** A section's statements are its in-use entries apiece (`Filed`: number, the section's
place in read order, location — 24 bytes, the old tuple's size, the place sitting in its padding) and its free
entries as `FreeRun`s, a consecutive run being one value however long. The table keeps the in-use winners in its
map and the free numbers in `FreeRuns`, disjoint runs. §7.5.6's "most recent copy" now holds between a run and an
entry: `fill` paints each run over the numbers no newer run covers, and an in-use entry survives unless a run read
before it covers its number. When every free run is from the oldest section with in-use entries or older — every
single-section file and ISO 32000-2 — that test is skipped. One section's runs are already ascending and disjoint,
so painting is skipped for them too.

**In-use entries have a named budget**, `xref::MAX_XREF_ENTRIES` = 2²³ across the whole chain. ISO 32000-2 sets no
number (its Table C.1 says only that object numbers "can often be expressed within 32 bits"); ISO 32000-1's Table
C.1 did — "indirect object 8,388,607 Maximum number of indirect objects in a PDF file" — and 2²³ is that many objects
and object 0's entry, 83 times the standard's own file, some 400 MiB at worst. A stream's in-use records are counted
from their type bytes before anything is held, so a stream past the budget costs its decode and no more; a classic
section is bounded by its own bytes and checked after reading. Past the budget the file is refused:
`SyntaxError::LimitExceeded { at: <section offset>, limit: "MAX_XREF_ENTRIES" }`. It is not rebuilt by a scan,
which would read a different document, and not read up to the cut, which would decide which objects exist.
`object_stream_pairs` refuses an object stream whose `/N` is past the same budget, as it refuses one it cannot
decode.

A constant rather than a `Limits` field: `Limits` is a public struct written out field by field in tests, fuzz
targets and `pdf-vfs`'s wire format, and the chain's other bound, `MAX_XREF_SECTIONS`, is a constant already.
`LimitExceeded::limit`'s doc now says a bound `Limits` does not carry is named by its constant.

## 3. Not built: stopping the decode at `/Index`'s rows

Table 17's `/Index` and `/W` fix how many bytes are entries, and the audit proposed decoding no further. The decoders
have no "stop at n bytes" answer: `max_stream_len` lowered to the stated length would turn a stream with bytes past
its rows, which reads today, into a refusal (`FilterRefusal::TooLarge`) and a scan, and a bounded pump does not run
the PNG predictor every cross-reference stream carries. The decode stays under `max_stream_len`, the budget every
stream has. After this change the bomb's whole cost is that decode: **12.7 MiB** for the 10 MiB its rows inflate to.

## 4. Cost on the launch path

Instructions to open ISO 32000-2, release, callgrind, one open: 63 169 707 before, **62 820 888** after (−0.6%). The
program's own share is flat: the type-byte count (+0.46 M) pays for itself in the smaller sort (−0.38 M, no free
entries in it) and allocator calls. A first version that sorted a 32-byte `(number, rank, location)` and painted
every run into a `BTreeMap` cost +9.4%, which is why the place in read order sits in the padding and painting is
skipped for ascending runs.
