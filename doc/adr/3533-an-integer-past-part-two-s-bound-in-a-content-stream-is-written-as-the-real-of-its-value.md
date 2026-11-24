# 3533 — An integer past part 2's bound in a content stream is written as the real of its value

Status: **accepted**, implemented (`crates/pdf-transform/src/archive/integer_reals.rs`).
Session 2208. Builds on ADR 3519 (the reading of `475121`), ADR 0816 (the fence), ADR 1176 (the
digit §7.3.4.3 assumes, the first content-stream byte written losing nothing), ADR 3393 (the
lexer's integer past `i64`), ADR 3506 (the repair log's rows). Context: ISO 32000-2 §7.3.3;
ISO 19005-2 section 6.1.13 (paraphrased).

## Context

Section 6.1.13 bounds integers at ±2^31 and reals at ±3.403×10^38, inside content streams as well
as objects. `475121` writes forty 34-digit integers (about 2.4×10^33) as `m` and `l` operands on
its one page, so at 2b the row `implementation-limits/values-written-in-content-streams` refused it
and part 4 held it. The reader's log also names the page for each (`syntax/integer-past-i64`), and
ADR 3506 maps that class to the objects' row `implementation-limits/integer-values`, so that row
failed beside it.

## Decision

1. **The period.** ISO 32000-2 §7.3.3 makes an integer and a real two spellings of a mathematical
   number, spells the real with a leading, trailing or embedded period, and says:

   > Wherever a real number is expected, an integer may be used instead.

   So `2366454844678207931159090160467968.` is the number its digits name, within the real bound.
   Where every value a content stream states past a bound is such an integer, and each is a direct
   operand of an operator of Table 50 whose operands are numbers (`NUMERIC_OPERATORS`), a period is
   written after its last digit and no other byte moves: `Mechanical(IntegerWrittenAsReal)`,
   losing nothing. An operand where §7.3.3 expects an integer (`J`, `j`, `Tr`), one inside an array
   or a dictionary, one before no operator, one past the real bound as well, and any real, string
   or name past its bound keep the target's refusal (`IMPLEMENTATION_LIMITS`), unchanged.
2. **The fence.** It is ADR 0816's own test, mark provenance: no operator is added, moved or
   respelled and the value is the same number, as ADR 1176's digit is. It is named in
   `doc/pdf-a-conversion-limits.md` section 5.2 beside `A50`, `A978` and that digit; no question
   was needed, since it takes less than either answer ratified.
3. **The log's row.** Where every place `integer-values` fails at is that log class at a page whose
   content this answers, and the page's own dictionary states no value as large as a 64-bit
   integer (`states_a_wide_value`), the same rewrite answers it (`answers_the_log`). The mapping in
   `pdf_archive::repairs` is unchanged. Section 5.1's answer over the same log (ADR 3521) counts
   the class answered where this rewrite writes every content stream of the page or form the log
   names, as it does the run rewrite's classes (`base_standard::settled`).
4. **Walked, then settled.** The streams are walked before the photographs are known and written
   after (`Walked::settle`): a stream a photograph takes off its page is left out, and where every
   stream stating a value past a bound was taken off, the photograph answers both rows at the
   losses it already takes (ADR 3535). A stream another content repair writes is refused by name.

## Consequences

- `475121` converts at 2b under `as-shown` and `as-if-printed`, 40 integers restated; our checker
  and veraPDF 1.30.3 pass both outputs. Byte proof: the output's decoded stream is the source's
  311254 bytes with 40 periods inserted, each after a run of 34 digits. Pixel proof: this tree's
  rendering of page 1 at 100 dpi is byte-identical PNG for source and output, and mupdf's too.
  Poppler 25.03 draws 805 pixels differently between the integer and the real form of the same
  file: evidence about poppler's reading of an integer past its range, not a target.
- Under `state-every-loss` it stops on `image-smoothing`, the profile's own answer.
- Tests: the module's unit tests; `crates/pdf-transform/tests/archive/integer_reals.rs`.
