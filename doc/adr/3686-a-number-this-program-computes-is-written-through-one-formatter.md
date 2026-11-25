# 3686 — A number this program computes is written through one formatter, which refuses inf and NaN

Session 2246. Status: accepted. Context: ISO 32000-2 §7.3.3; ADR 3683; record 2245's census.

## Decision

`pdf_syntax::write::real_number` (`f64`) and `real_number_f32` write §7.3.3's real — an integral
value as an integer, any other as the shortest decimal that reads back as the same value, never an
exponent — and answer an infinity or a NaN with `write::NonFinite`. §7.3.3 has a real as "one or
more decimal digits optionally preceded by a sign and containing a period"; Rust's `inf` and `NaN`
are neither, and read back as names. `write::object`'s own `real` is the same formatter, keeping
`0` for a synthesised `Object::Real` because its callers cannot refuse.

`real_number_f32` prints what `{}` printed for every finite `f32`, so no output's bytes change.

## Each writer

- `appearance.rs`'s `Stream` (`cm`, `g`/`rg`/`k` and stroking forms, `w`, `d`, `m`, `l`, `c`, `re`,
  the icon's `cm`): every operand through `Stream::real`; the first refusal is held and the
  appearance is not drawn, reported as `Refusal::NonFinite`. `/Rect` edges are finite (`number`),
  and their difference can still overflow `f32`.
- `colour_operator`: a script's colour was clamped, and NaN passes `clamp`, so `["RGB", "x", 0, 0]`
  wrote `NaN 0 0 rg` into a `/DA`. `pdf-script`'s `colour_of` now answers no colour for a
  non-finite component, and the operator goes through `real_number`.
- `variable_text.rs` (`re W n`, `Tf`, the selection's `m`/`l`, both `Tm`s, `show_adjusted`'s `TJ`
  numbers) and `rich_text/layout.rs` (`re W n`, `rg`, `re`, `Tf`, `Tc`, `Tz`, `Tm`, the `/DA`'s
  `Tf` and `rg`): through `variable_text::operands`; a refusal is `Owed::NonFinite`, and `lay_out`
  writes no appearance.
- `redact/overlay.rs`'s `/RO` origin (`f64`, a `/Rect` corner less a `/Matrix` translation): through
  `real_number`; past `f64` the overlay is not drawn and the departure says why.
- `view/markup.rs`: no hand-formatted real; its numbers are `Object::Real`s.

## Left

`pdf-transform/src/archive/` (`derived.rs`'s matrix, `preserve.rs`'s and
`fonts/photographed.rs`'s `number`) belongs to another lane this session and is not edited; each
is to go through `real_number` the same way.
