# 1597 — A field's one-call script is fuzzed, and a date picture is read in one pass

Session 1380. Status: **accepted**. Supersedes nothing. Context: ADRs 1578 and 1579 (Tier 0's
library and its dispatch); `CLAUDE.md` principle 3 (fuzzing from the first parser commit; time
budgets); ISO 32000-2 §12.6.3's Table 199; traps 107, 111 and 112.
Code: `crates/pdf-model/src/aform/date.rs` (`tokens`, `LONGEST_PLACE_HOLDER`).
Tests: `aform::date::tests::a_picture_of_one_repeated_letter_is_read_in_one_pass`;
`fuzz/fuzz_targets/aform.rs` (new), `fuzz/seed_aform.py` (new), its `fuzz/seeds.sh` arm and its
`doc/verify.md` line.

## 1. Why a target of its own

Since Tier 0 a producer's `/JS` text reaches `Call::parse` and the twenty-one `AF*` functions on
every keystroke, format, validation and calculation a field raises. These are untrusted bytes on a
new path. `page` and `forms_data` open documents and raise no trigger, so neither reaches the
parser. `aform` takes three NUL-separated texts: the script, the field's value and the keystroke's
change, with the selection taken from the value's first two bytes. It runs the call at all four
triggers and the helpers on the value. It checks two properties beyond not panicking. A script
that parses once parses again to the same call. A keystroke example the call offers is one it
commits. The target drives `pdf_model::aform` directly rather than through a document: the
dispatch from `/AA` is `view`'s, and a document around the script would spend the fuzzer's
mutations on the object graph.

`seed_aform.py` writes 350 seeds. They cover every function name, under each argument shape the
grammar takes and against a value of the kind each function reads. They also cover the malformed
cases the contract named: unterminated strings, numbers of four hundred digits and past the double
range, quotes nested in the other kind, ten thousand arguments, and scripts at and past the 64 KiB
bound.

## 2. The defect it found

`tokens` counted the run of a repeated letter to its end at every token, then took at most four.
A picture of one letter repeated n times therefore cost about n²/8 comparisons. The seed
`AFDate_FormatEx("mm…")`, 64 000 letters and inside `MAX_CALL_BYTES`, cost **991 ms a call** in
the release fuzz binary and 0.70 s in the unit test's profile. A field's format and keystroke
scripts are called on every keystroke and every redraw. The count now stops at
`LONGEST_PLACE_HOLDER` (four, for `mmmm`, `dddd` and `yyyy`), and nothing a token reads needs
more. The same seed is 9 ms with process start-up. The target's rate rose by the same cause, from 0.47 M executions in twenty minutes to 40.7 M. The test holds the tokeniser to 250 ms on that
picture and failed at 0.70 s with the count unbounded. No input crashed the target.

## 3. The campaign (one process per target, `-s none`, `tools/bounded.sh`, behind the lock)

Every target ran from seeds written fresh by `fuzz/seeds.sh` into a scratch root, never into the
main checkout's corpus, with `-max_total_time=1200` and `doc/verify.md`'s limits. No `-fork`,
`-jobs` or `-workers` was used. `page`'s line states `-fork=6`, and this campaign ran it as one
process. INITED → final coverage:

| target | seeds | edges | execs | artefacts |
|---|---|---|---|---|
| `page` | 15 234 | 31 050 → 34 241 | 95 844 | one slow unit (section 4) |
| `forms_data` | 21 | 1254 → 2589 | 19.1 M | none |
| `xfdf` | 20 | 1912 → 3650 | 78.3 M | none |
| `fetched_import` | 136 | 6746 → 10 053 | 0.85 M | none |
| `aform`, before the fix | 350 | 1366 → 1896 | 0.47 M | none |
| `aform`, after it, from the first run's corpus | 1391 | 1899 → 2103 | 40.7 M | none |

## 4. What was left

`page`'s slow unit, `slow-unit-2ecfe415…` (2440 bytes, 2.8 s in a release `callgrind_interpret`),
is a seed and not a mutation. Its Type 3 glyph draws a second Type 3 font, whose glyph fills with
a tiling pattern that shows the first font again. That is the Type 3 cycle `doc/todo/49` already
lists among the `page` target's six slow shapes. It is bounded by the nesting depth and the
tiling's copies, and it is not this decision's.
