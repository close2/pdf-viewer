# 3683 — A default appearance is restated in §7.3's syntax: names escaped, reals at the lexer's value

Session 2245. Status: accepted. Context: review 2243 (B05, B06), ISO 32000-2 §7.3.3, §7.3.5,
§12.7.4.3, Annex C (informative); ADR 0453.

## Decision

- **Names.** The `Tf` that `appearance::scripted_appearance` restates from the producer's `/DA`
  keeps the font as a `pdf_syntax::Name` and writes it through `Name::escaped`. The bytes never pass
  through a `String`, the same as `variable_text`'s own `Tf` (ADR 0453). `/Helv#20Bold` is restated
  as itself, not as `/Helv Bold`.
- **Reals.** `variable_text::real_operand` writes a real through `pdf_syntax::write::object`'s
  formatter: the shortest decimal that reads back as the same `f64`, and never an exponent. The
  `/DA` replay writes the `f64` the lexer read, not the `f32` the layout uses. The restated `Tf`
  size is the operand as stated (`DefaultAppearance::size_as_stated`).

## What a value past the range becomes

Annex C's Table C.1 is informative, so a real past `f32`'s range is still §7.3.3 syntax. It is now
written as the number it is, where before it was written `inf`. A literal past `f64`'s range never
reaches the writer as an infinity: the lexer holds it at `±f64::MAX`, and it is written as that
many digits. The formatter's `0`, its answer for a value §7.3.3 cannot spell, is reached only by a
caller's own infinity or NaN. A script's `textSize` is refused outside 0 to 32 767 before it gets
that far.

## Census

No hand-formatted writer in `pdf-model` or `pdf-transform` writes a name taken from a document
without `Name::escaped`. Every raw `/`-and-bytes site writes a name the program generated. About
twenty sites write reals with Rust's `{}` or `{:.N}`, which never print an exponent but print `inf`
or `NaN` for a non-finite value. They are listed in the record as risks for non-finite input, not
fixed here.
