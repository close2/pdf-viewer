# 757 — The widths a stream states once, and every record re-read

General-improvement round, chosen rather than assigned. Subject: **the *composition* behind
`doc/todo/42` §1's `Document::open` floor** — a sentence written in the two-hundred-and-seventy-sixth
session whose total was re-taken four rounds ago and whose breakdown was not. Re-running the
attribution found the ranking inverted, and the largest item in the profile was a function nothing
in this tree had ever named. Taken: **−10.49% of `Document::open`, byte-identically**.

ADR **0677**. 0678 and 0679 unused.

## Why this subject

The briefing's rule was that the highest-yield general round finds a number this project wrote down
and has not re-run, and that the best of them cost nothing to set up. Three siblings were on the
native hosts' drawing thread, the errata rule's fourth step and the oracle's *we are alone* border,
so those were out.

`doc/todo/42` §1 fitted the rule with a twist that turned out to be the finding. Its sentence —
"76.6 M instructions, of which inflating the two cross-reference streams is 18 M and nothing can
remove it, so the remaining ceiling on this route is roughly a further 40%" — has a **total** that
ADR 0667 re-took four rounds ago (67.82 M) and a **composition** nobody had. The instrument needed
no edit at all: `crates/pdf-syntax/examples/callgrind_open.rs` has existed since ADR 0180 and is one
`valgrind` invocation.

The first run reproduced ADR 0667's figure to the instruction (678,200,421), which is the check that
the tree is where the previous round left it, and then said the ranking had inverted:
`xref::read_section` **37.0%** against the predictor's 15.2% and zlib's 7.6%. The clause the sentence
called irreducible is under a quarter of the open, and the largest single item was unnamed.

## What was found

`#[inline(never)]` on one function attributed it exactly: **`entry_location` was 18.4 M per open, a
quarter of `Document::open`, to read three integers out of a seven-byte record** — 164 instructions
apiece. It was re-deriving Table 18's three field offsets from `/W` for every one of ISO 32000-2's
101 318 entries, and accumulating each field a byte at a time under a saturating multiply.

§7.5.8.2's Table 17 says `/W` describes the *stream* — "[t]he sum of the items shall be the total
length of each entry" — so the offsets are the same for every record in it. `RecordLayout::of`
resolves them once, beside the code that already computes `row` from the same array.

## What moved

| | before | after | |
|---|---:|---:|---|
| `callgrind_open`, ten opens of ISO 32000-2 | 678,200,421 | 607,023,715 | **−10.49%** |
| `callgrind_interpret`, page 101 ×50 | 1,285,546,279 | 1,278,428,629 | −0.55% |
| `callgrind_rasterise`, page 101 | 5,431,961,793 | 5,420,432,898 | −0.21% |

Both arms built and run in one sitting with identical arguments; two passes each, every pass
identical to the instruction. `read_section` drops 71,176,710 against a total of 71,176,706, and
every other row of the profile is unchanged — so the attribution is the whole delta. The interpret
line's −7.12 M is exactly one `Document::open`, which is what that example does once.

**One baseline had moved and re-deriving it mattered**: ADR 0667 left `callgrind_rasterise` at
5,450,359,321 and this tree's own before-arm is 5,431,961,793, so quoting the previous round's
figure would have turned −11.5 M into −29.9 M.

## Byte-identity, and trap 13

A temporary differential held the previous `entry_location` verbatim beside the new one over 200 000
generated cases — each `/W` width independently 0 to 12, record lengths from zero to three past the
row, bytes biased to `0x00` and `0xff`. **0 disagreements**, and three planted defects caught 4 440,
833 and 2 367 cases before that zero was believed. Not committed, for ADR 0667's reason.

## The choice the split made visible

Separating the ≤ 8-byte path from the wider one turned an undocumented behaviour into a decision
that had to be stated: §7.5.8.2 bounds no element of `/W`, so a field wider than a `u64` is a thing
a file may write and the standard says nothing about a value that does not fit. It has clamped since
the code was written, nothing said so, and nothing tested it.
`a_field_wider_than_a_u64_clamps_rather_than_wrapping_round_to_a_plausible_offset` pins it — and the
assertion that discriminates is not whether the object is found, because `Document::load_by_header`
repairs the entry either way, but whether `misfiled_objects()` **names it**. Calibrated by planting
the wrap.

## Gates and load

The full §2 sequence, `pdf-syntax` being in the everything row: fmt clean, `RUSTFLAGS="-D warnings"
clippy --workspace --all-targets` clean, 2664 tests, doctests, `fuzz/ --bins` checks, corpus,
oracle, text extraction ×3, both censuses, dates, xmp, JPEG 2000, quorra, fixed documents,
conformance — all green, no ratchet moved. The machine carried three parallel rounds at load 22 to
26 throughout; nothing here is a wall clock.

§5's binaries were **not** rebuilt: this is not a fifth round, and the measurement is callgrind on a
purpose-built `--release` example, which is what `doc/verify.md` prescribes and which names none of
§5's six. `tools/round.sh` reports `target/` empty in this worktree, which is the worktree's state
rather than this round's.

**`PDFREF_CACHE` was left unset, which was a mistake and is recorded as one.** The briefing asked
for the shared warm cache; unset, the oracle, text and selection lines fall back to
`<target-dir>/tmp/pdfref-cache`, and a worktree has its own target directory — so this round paid
`pdftoppm`, `mutool` and `gs` to rebuild **360 MB** of references beside the shared 2.5 GB copy at
`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`. No verdict is affected: the cache saves
invocations rather than deciding anything, and the gates were green either way. What it cost is
reference-renderer time on a machine carrying three other rounds — which is the load §2 warns
about, spent by the round that was warning about it. `PDFREF_CACHE=/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`
is the whole fix, and it belongs in the shell before the first gate rather than in a note
afterwards.

## Files

`crates/pdf-syntax/src/xref.rs`, `crates/pdf-syntax/tests/cross_references.rs`,
`doc/conformance/ledger.toml` (§7.5.8.2 and §7.5.8.3), `doc/todo/42-the-launch-path.md`,
`doc/habits.md` (two measuring habits), `doc/adr/0677-…`.
