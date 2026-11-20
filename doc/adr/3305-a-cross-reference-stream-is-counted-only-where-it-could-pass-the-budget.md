# 3305 — A cross-reference stream is counted only where it could pass the budget

Session 2151, a measure round in the `lane3` worktree. Status: **accepted** and built. Context: ADR 3257 (free
entries are runs, in-use entries have a named budget), ADR 3273 and ADR 3274 (a filter parameter and a chain read as
stated), ADR 3262 and ADR 3285 (the launch bands), ADR 2666 (the counted open figure); `doc/checks/launch-path.toml`;
`crates/pdf-syntax/src/xref.rs` (`read_xref_stream`, `decode_direct`), `crates/pdf-syntax/src/document/decoding.rs`
(`chain_of`). Clauses: ISO 32000-2 §7.4, §7.5.4, §7.5.8.2.

## 1. What grew

`launch_path` left three counted opens above their bands after round 2146: ISO 32000-2's sponsored EC3 edition
56 427.4 thousand (55 063 .. 56 176), `PDF20_AN001-BPC` 2 227.4 (2 179 .. 2 224), `xfa_filled_imm1344e` 287.2
(279 .. 286). Round 2140's profiles of `2c1e7568`, which has neither `3c3ed860` nor `4752821f`, were diffed by
function against HEAD's (callgrind, this program's object, release under Rust 1.100.0):

- **The closure that reads a cross-reference stream** (`read_section`'s, into which `read_xref_stream` inlines) grew
  by the pass ADR 3257 added: `in_use_records` reads every record's type byte before any entry is held, so that a
  stream past `MAX_XREF_ENTRIES` is refused having cost its decode and no more. On EC3 (110 219 + 2 050 records) the
  pass is 2.36 M instructions, 4.2 % of the open; on AN001 5.4 thousand, on the XFA form 2.3.
- **A stage's parameter dictionary was copied twice** by `chain_of` (into the per-stage vector, then out of it by
  `cloned()`), and `decode_direct` copied `/Filter` and `/DecodeParms` out of the stream's dictionary before
  borrowing them. Both arrived with ADR 3274's one reading of the chain.
- The sorts did not grow: the `Filed` sort and the map's bulk build cost less than the `(u32, Option<Location>)`
  ones they replaced, the table holding no free entries.

## 2. The decision

**The pass runs only for a stream that could be past the budget.** No record is read beyond the `/Index` total or
the end of the decoded data, so a stream's in-use entries are at most the fewer of those two. Where that number is
within what is left of the budget, the refusal cannot be reached, and the number is the capacity instead; where it
is not, the stream is counted as before and refused or read by the count. ADR 3257's property — a stream past the
budget costs its decode and no more — is unchanged, and one allocation still holds every entry.

The cost, written down: the capacity now includes the stream's free records, which are never written. That is
address space, 24 bytes a record, bounded by the budget (some 192 MiB at 2²³ records, where the budget's own worst
case is that much resident), and no resident memory: the audit's 2 097 152-free-entry file
(`free_entries_do_not_cost_the_reader_memory_apiece_spec_audit`) measures `VmHWM`, which untouched pages do not
raise. A stream of more records than the budget whose entries are free is read through the counted path
(`a_stream_of_more_records_than_the_budget_whose_entries_are_free_is_read`).

**A parameter dictionary is moved to its stage**, not cloned a second time, and the cross-reference stream's chain is
read from borrowed values.

## 3. Measured

Release builds of `launch_path`, `count-open` under callgrind, this program's object, thousands: EC3 56 427.4 →
54 067.8, AN001 2 227.4 → 2 220.1, `xfa_filled_imm1344e` 287.2 → 284.8, WTPDF 10 177.0 → 10 107.6, `bug1815476`
1 768.2 → 1 764.2, `opt_demo` 406.8 → 407.1. Of EC3's 2 359.6, the counting pass is 2 357.9.

## 4. The bands that move

EC3's open falls below its band's lower edge, which ADR 2915 and ADR 2950 had set around the compiler's code for the
same closure, and its first frame, which counts the open, falls with it by the same 2.35 million. Both move by the
file's rule (the observed figure widened by one per cent, rounded outwards) around the gate's own children: the open
54 067.8 thousand, 55 063 .. 56 176 → 53 527 .. 54 609; the first frame 125 705.2, 126 777 .. 129 339 →
124 448 .. 126 963. No other band moves; AN001's and the XFA form's opens return inside theirs.
