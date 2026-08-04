# ADR 0180 — §7.5.6's rule stated once for a file, not once per entry

Status: accepted, 2026-08-04 (session 276).

## Context

ADR 0179 measured the launch path and found `Document::open` costing 12 to 22 ms on ISO 32000-2 —
1023 pages, 101 318 objects — against 0.20 ms on a five-page file, where `CLAUDE.md` says "a
500-page document must open no slower than a 5-page one". [todo 42](../todo/42-the-launch-path.md)
recorded it as not localised.

`crates/pdf-syntax/examples/callgrind_open.rs` localises it. Wall-clock on this machine moves by a
factor of two between runs of the same binary, so the instrument is callgrind: instruction counts,
ten opens per run, and the harness's own costs kept out of the loop (an `Arc<[u8]>` cloned rather
than a `Vec`, and the trailer counted rather than the 101 318 entries).

**Per open, before anything:** 138.3 M instructions. And 40% of it was one line.

| | Ir per open | |
|---|---|---|
| `btree/search.rs`, `cmp.rs`, `btree/node.rs`, `entry.rs` | **52.3 M** | the cross-reference map |
| `xref::read_section` | 25.9 M | the entry loop |
| `filter::decode_with_parms` | 18.6 M | inflate and the PNG predictor |
| `calloc` + `free` | 10.6 M | one row buffer per predicted row |
| everything else | ~30 M | |

## The finding

`XrefTable::add` was `self.entries.entry(number).or_insert(location)` — one searched insert per
cross-reference entry, into a `BTreeMap<u32, Option<Location>>`. It is the *right* statement of
§7.5.6's rule:

> the most recent copy of each object shall be the one accessed from the PDF file

Sections are read newest first, and an entry that is already present is never overwritten. But the
rule is a property of the *whole file*, and it was being re-decided 101 318 times, each decision
costing a seventeen-level descent through a tree that was being built at the same time.

## Decision

**Collect every section's entries in read order and file them once.**

```rust
fn fill(&mut self, mut entries: Vec<(u32, Option<Location>)>) {
    entries.sort_by_key(|(number, _)| *number);
    entries.dedup_by_key(|(number, _)| *number);
    self.entries = entries.into_iter().collect();
}
```

Three lines, and each carries a clause:

- **The order of `entries` is the precedence.** Newest section first, a hybrid file's `/XRefStm`
  between its own section and the older one it precedes (§7.5.8.4), older sections after — which
  is exactly the order `read_from_startxref` already visited them in.
- **The sort is stable**, so among entries for one object number the newest section's stays first.
- **`dedup_by_key` keeps the first of each run**, which is `add`'s "first writer wins" applied to
  a whole file in one pass. A free entry is a `None` and is kept, so §7.5.6's deletion still
  overrules the older section's offset — the thing ADR 0100 established and the reason an entry is
  an `Option` at all.

It is also *faster than it looks*, because of what the inputs are: a section's entries are already
ascending, and there are two sections, so Rust's stable sort sees two runs and merges them in one
pass. `BTreeMap`'s `FromIterator` then bulk-builds from sorted input rather than searching for
each key.

**130.7 M instructions per open before, 76.6 M after — 41%.** Wall clock on the same file, seven
runs: a median `Document::open` of 13.6 ms became 9.8 ms, and the viewer's own launch timeline
moved from `document open +27.8 ms` to `+21.0`.

## Two smaller ones in the same sitting, both allocation

- **`apply_predictor` allocated a row buffer per row.** A PNG image has a few thousand wide rows
  and would not notice; a cross-reference stream has one *six-byte* row per object, so ISO
  32000-2's put 101 318 `calloc`/`free` pairs on the launch path. Two buffers, swapped. 138.3 M →
  131.3 M, **5.1%**, and all of it in libc rather than in the loop, which is why the loop itself is
  untouched.
- **`read_xref_stream`'s entry `Vec` grew by doubling**, seventeen times to reach 101 318 entries.
  Its capacity now comes from the decoded data's own length — a bound a malformed `/Index` cannot
  inflate, because the data has been materialised already. 0.4%, kept because a length that is
  known is a length worth stating.

## What did not change

Every verdict. The corpus gate, the oracle's 1794 pages, the text gate's 98.2%, the date gate and
the nine `pdf-syntax` suites are unmoved — which is the check that matters for a change to the one
structure every object resolution goes through. §7.5.6's and §7.5.8.4's ledger rows both named
`XrefTable::add` in their notes and now name `fill`; the second of them explains the hybrid
precedence *by* the order two loops ran in, which is now the order one list was appended in.

## The lesson

**A rule that is true of a file can be enforced once for the file.** The per-entry form was not
wrong and was not slow-looking — `or_insert` is the idiom, and the cost is invisible until a
document has a hundred thousand objects. What made it findable is that `CLAUDE.md` states a
*ratio* as the requirement — a big document must open like a small one — so the instrument compares
two documents rather than watching one number.
