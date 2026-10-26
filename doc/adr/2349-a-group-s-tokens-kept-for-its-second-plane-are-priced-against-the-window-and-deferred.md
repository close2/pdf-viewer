# 2349 — A group's tokens kept for its second plane are priced against the window, and deferred

Status: deferred — priced and not built; section 5 names what reopens it. Session 1757, 2026-10-26,
the measure slot of batch one hundred and fifteen. Context: ISO 32000-2 §7.8.2, §11.4.7; ADRs 0362
and 0365 (the window a content stream is read through), 1645 (the black list is a second run),
2244 (what the launch gate's memory figure holds), 2332 section 5 (the lever as first priced);
habit 164. Moves no ledger row and no band.

## 1. The lever, and the brief's premise that did not hold

ADR 2332 section 5 left one lever on `bug1721218_reduced.pdf`: its `DeviceCMYK` group form, 2.8 MB
inflated, is read once on each of §11.4.7's two process planes, and keeping the first run's tokens
for the second would save that reading. The brief asked for the hold's price per page and its
lifetime against the memory high-water gate before building it, "behind the page cache's own
eviction (ADR 2244 names what the cache holds)". **That premise did not hold**: the cache ADR 2244
names is the kernel's — the file-backed pages of the test binary and the C library that the launch
gate's `open_anon_mib` was made to leave out — and no cache of this program holds a content
stream's tokens. A kept run would live inside one interpretation, from the chromatic run's first
token to the black run's last (`black_half` in `content/transparency/group_runs.rs`), and nothing
would evict it but the end of the group.

## 2. The hold, counted

The group's content, inflated, is 2 804 559 bytes: 324 596 numbers, 10 723 names, 50 strings —
**335 369 operands**, not the "some 600 000" ADR 2332 estimated — and 134 410 operators. An operand
is a `pdf_syntax::Object`, 32 bytes on this target (measured with `size_of`), so the operands alone
are 10.7 MB, a name's shared bytes about 0.3 MB more, and a record of each operator with its
operands' range about 1.6 MB: **about 12.7 MB held**, four and a half times the content's own text.
Interpreting the page today peaks at 23.4 MiB resident in `examples/callgrind_pages`, where a
one-page document with no group peaks at 11.8: the hold would more than double what this page's
interpretation asks for. A compact record — each number as its tag and its eight bytes, each name
as an index — would be about 3.2 MB, still fifty times the 64 KiB window.

## 3. What the memory gate can see of it

Nothing. `launch_path` bands `peak_anon_mib` and `open_anon_mib` on six documents, and none states
a group whose blending space has four components: ISO 32000-2's 1 018 groups are all `/DeviceRGB`,
the Well-Tagged PDF's eight are `ICCBased` with `/N 3`, and the other four state none. `turn_path`
bands no memory figure. So "the hold clears the memory gate on the gate's documents" is true only
because the gate cannot see the hold at all (trap 13); the gate is not the instrument for it.

## 4. Why it is not built

ADR 0365 reads every content stream through a window so that a stream's inflated length is time
and never an allocation: a 1.85 MB file inflating to 1.77 GiB costs 8.6 MB. A kept run of tokens
is that allocation back, for every group whose space has four components, at four and a half times
the inflated length. Building it safely is a design of its own — the record charged to
`MAX_LIST_BYTES` as it grows, given up and re-read when the charge fails, and its peak banded on a
gate document that has the construction — and the saving it buys is, at its ceiling, the second
run's lexing and inflation (ADR 2332 section 5's 170 M instructions, re-derived in section 5 as
about 155 M) on the one page the turn gate holds that has it.

## 5. The price, re-derived, and what reopens it

The ceiling, from callgrind on HEAD's export (`examples/callgrind_pages`, a run of two
interpretations less a run of one, threads apart): the interpreting thread is 869.9 M instructions,
ADR 2332's figure. Of it, reading the two runs' tokens is about 277 M — `content_token_at_cursor`
166.4 M, the operand step 54.6 M, `token_to_object` 24.9 M, the operands' drops 22.3 M,
`Operator::of` 5.8 M and the lexer's own 3.2 M — and inflating the forms about 50 M; the
operators' dispatch, 144.4 M, runs on each plane whatever is kept. One run's reading is therefore
about 164 M, and a replay still makes and drops each operand, about 10 M: **the ceiling is about
155 M, 18 % of the thread**, which at the thread's 49 ms in the turn gate is about 9 ms of a 130 ms
turn — on one page, for 12.7 MB held.

Reopened by either: a census that finds the four-component pair on pages a person turns often
enough to matter, or a content budget that already charges kept tokens for another reason, at
which point the record above costs a charge and not a design.
