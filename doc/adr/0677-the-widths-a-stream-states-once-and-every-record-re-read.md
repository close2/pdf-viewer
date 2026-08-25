# ADR 0677 — The widths a stream states once, and every record re-read

Status: accepted, 2026-08-25. Session 757. §7.5.8.2's `/W` is resolved into byte ranges once per
cross-reference stream instead of once per entry: `Document::open` of ISO 32000-2 falls **10.49%**,
byte-identically. Clauses: ISO 32000-2 §7.5.8.2 and §7.5.8.3, whose ledger rows move with it.

## How this was found, and it is the same shape as ADR 0667's

`doc/todo/42` §1 has carried one sentence since ADR 0180, four hundred and eighty rounds ago:

> The measured floor now: 76.6 M instructions, of which inflating the two cross-reference streams
> is 18 M and nothing can remove it, so the remaining ceiling on this route is roughly a further
> 40%.

The **total** in that sentence was re-taken four rounds ago (ADR 0667: 76.33 M, then 67.82 M after
the predictor came out of the per-byte loop). Its **composition** was not, and the composition is
the part that decides what to do next. Re-running the profile the sentence was written from says
the ranking has inverted:

| per `Document::open`, ISO 32000-2 | Ir | share |
|---|---:|---:|
| `xref::read_section` — Table 18's records | **25.11 M** | **37.0%** |
| `filter::decode_with_parms_reported` — §7.4.4.4's predictor and the chain | 10.32 M | 15.2% |
| `zlib_rs::inflate` | 5.13 M | 7.6% |

So the inflation the sentence calls irreducible is under a quarter of the open, and the largest
single item is the one nobody had named. ADR 0667 wrote of its own find that the predictor "sat
underneath that measurement, unnamed, for four hundred rounds"; this is the next stratum down, and
it was made visible by the same round's change rather than by anything new.

**`#[inline(never)]` on one function is what attributed it**, and that is worth keeping as a method:
under a fat link `entry_location` is inlined into `read_section` and callgrind can only report the
sum, with the inlined library code filed under `uint_macros.rs`, `take.rs` and `slice/index.rs`
against `read_section`'s name. One temporary attribute, one rebuild, and the answer is a row of its
own: **18.4 M per open — a quarter of `Document::open` — inside a function that reads three
integers out of a seven-byte record.** 164 instructions per record.

## What the loop was doing

Table 18's record is three fields whose widths `/W` gives, and §7.5.8.2's Table 17 says of `/W`:

> The sum of the items shall be the total length of each entry; it can be used with the Index
> array to determine the starting position of each subsection.

So `/W` describes the **stream**: every record in it has the same three fields at the same three
offsets. `entry_location` was re-deriving those offsets from `/W` for every record — a
`widths.iter().enumerate().take(3)`, a running `at.saturating_add(width)`, a `record.get(range)`
and a `fields.get_mut(slot)` apiece — and then accumulating each field a byte at a time with
`value.saturating_mul(256).saturating_add(byte)`, which is a clamp on every byte of every field of
every entry in the file.

ISO 32000-2 has 101 318 objects, and this is the population that cares. It is the same sentence
ADR 0180 already wrote about the comment two lines above it, and the same one ADR 0667 wrote about
the predictor: **one small row per object, and a hundred thousand of them.**

## What changed

`RecordLayout::of` turns `/W` into three byte ranges once, beside the code that already computes
`row` and the entry capacity from it. `entry_location` walks those three ranges. The per-byte
clamp becomes a shift on the path every real file takes, and stays a clamp on the path no file
takes:

```rust
fn big_endian(bytes: &[u8]) -> u64 {
    if bytes.len() > FIELD_BYTES {
        return bytes.iter().fold(0u64, |value, &byte| {
            value.saturating_mul(256).saturating_add(u64::from(byte))
        });
    }
    bytes
        .iter()
        .fold(0u64, |value, &byte| value.wrapping_shl(8) | u64::from(byte))
}
```

**The two arms agree wherever both are defined**, which is what makes this a hoist rather than a
change of arithmetic: eight bytes or fewer cannot overflow a `u64`, so no clamp is reachable on the
first path and the shift is exact; past eight bytes the clamp is the old loop, verbatim.

Nothing about *which* bytes a field is made of moved, and nothing about Table 18's three entry
types moved. `RecordLayout::of` keeps the old `saturating_add` for the running offset deliberately
and says why in a comment: `checked_add` would refuse an absurd `/W` **here**, where the old code
refused it at the first record — and a stream whose `/Index` declares no entries reads no record,
so it was not refused at all. That is a difference of one malformed-file outcome, and it is not one
to make silently.

## What it is worth

Callgrind, `--profile release`, both arms built and run in one sitting with identical arguments,
two passes each and every pass identical to the instruction:

| | before | after | |
|---|---:|---:|---|
| `callgrind_open` — ten opens of ISO 32000-2 | 678,200,421 | 607,023,715 | **−10.49%** |
| per `Document::open` | 67.82 M | **60.70 M** | −7.12 M |
| `callgrind_interpret` — page 101 ×50 | 1,285,546,279 | 1,278,428,629 | −0.55% |
| `callgrind_rasterise` — page 101 | 5,431,961,793 | 5,420,432,898 | −0.21% |

**The attribution is exact.** `xref::read_section` goes 251,086,670 → 179,909,960, a drop of
71,176,710, against a change in the total of 71,176,706 — four instructions, which is the process's
own start-up and not the code. Every other row of the profile is unchanged to the instruction:
`decode_with_parms_reported` 103,172,400, `zlib_rs::inflate` 51,257,120, the `XrefTable::fill` sort
33,457,290, `drop_glue::<Document>` 17,826,020.

`callgrind_interpret`'s **−7.12 M is one `Document::open`**, which is exactly what that example
does once, so the two instruments agree with each other rather than merely both improving.
`callgrind_rasterise`'s −11.5 M is one open plus 4.4 M the open cannot explain; it is code layout,
the same effect ADR 0667 recorded in the other direction (+1.9 M) and for the same reason, and it
is recorded rather than explained away.

**The two baselines had to be re-taken and one had moved.** ADR 0667 left
`callgrind_rasterise` at 5,450,359,321 four rounds ago and this tree's own before-arm is
5,431,961,793 — 18.4 M apart, from the merges since. Quoting the previous round's figure as a
baseline would have turned a −11.5 M into a −29.9 M.

## How byte-identity was checked, and how the check was checked

A differential test held the previous `entry_location` verbatim beside the new one and compared
`Option<Entry>` over **200 000 generated cases**: each of the three `/W` widths independently 0 to
12 — spanning the absent field, the widths files use, and widths past a `u64` — record lengths from
zero to three past the row so short records are reached, bytes biased to `0x00` and `0xff` so the
leading-zero and clamping paths are exercised rather than merely reachable, and a random `base`.
**0 disagreements.**

Trap 13 says a sweep for a defect is not believed until it has been run against the defect, and
trap 746's lesson is that a plant has to be asymmetric enough to fail. Three plants:

| plant | caught |
|---|---:|
| a zero width reads bytes instead of taking Table 17's default | 4 440 cases |
| the clamp branch widened past the widths generated, so 9 to 12 bytes wrap | 833 cases |
| the field's bytes folded least significant first | 2 367 cases |

The restored tree returns 0 again. The differential is not committed — it carries a verbatim copy
of code that no longer exists, which is a thing to run rather than a thing to keep — and what is
committed is the permanent test below.

## The choice this made visible, which had never been written down or tested

§7.5.8.2 states no maximum for an element of `/W`, so a file may declare a field wider than the
`u64` this reader accumulates it in, and the standard says nothing whatever about a value that does
not fit. The code has clamped since it was written; nothing said so and nothing tested it, and
separating the two arms made it a decision that had to be stated.

**It is a choice and it is recorded as one** (principle 5): the value clamps rather than wrapping,
because `u64::MAX` is an offset past the end of every file and an object number no `u32` holds, so
a record stating one names nothing reachable — where the low 64 bits of the same number are a
plausible offset into a file the record does not describe.

`a_field_wider_than_a_u64_clamps_rather_than_wrapping_round_to_a_plausible_offset` pins it, and its
construction is the point. `/W [1 9 2]`, the same file twice, differing only in the leading byte
the ninth position makes room for: `00`, and the entry states the offset an eight-byte field would
have; `01`, and it states that offset plus 2^64. **Both arms still find the object**, because
`Document::load_by_header` repairs an entry the file's own `N G obj` header disproves — so the
assertion that discriminates is not whether the object is found but whether the repair happened:
`misfiled_objects()` is empty in the first arm and names object 4 in the second. A reader whose
arithmetic wrapped lands on the object, believes the table and reports nothing, and the test fails
on that exact row — checked by planting the wrap, which turns `[4]` into `[]`.

That is trap 5's rule reached from an unexpected side: the difference between clamping and wrapping
here is not what the reader *draws*, it is whether the reader knows it repaired something.

## What this does not do

- **It does not defer the table**, which is what `doc/todo/42` §1's remaining design question is
  about. Every entry is still materialised at open. What the item gains is a current denominator
  and a corrected ranking; what it loses is the claim that inflation is the irreducible core.
- **It does not touch the classic table**, whose entries are tokenised rather than sliced and
  which has no `/W`.
- **It does not touch the per-entry object-number arithmetic** —
  `u32::try_from(u64::from(first).saturating_add(u64::try_from(offset)…))`, still about 2.7 M
  per open in `convert/num.rs` under `read_section`. A faithful rewrite has to preserve the
  `continue` that advances the cursor without pushing an entry, and the early return that
  truncates a section when the data runs out; that is a behaviour question rather than a hoist,
  and it is not worth risking for a further ~3%.
- **It does not vectorise the byte fold**, for ADR 0667's reason: `#![forbid(unsafe_code)]` is the
  point of this crate. `u64::from_be_bytes` over a right-aligned buffer was tried and is **worse**
  — 646,939,755 against 607,023,715 — because a `copy_from_slice` of a runtime length becomes a
  call where the fold does not.
