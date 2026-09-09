# 0930 — What the file said, and what the reader kept

Session 941. Status: **accepted**.

## The problem

Two rows of `pdf-archive`'s requirement table were `Check::Unchecked` for the same kind of reason,
and neither reason was about ISO 19005. Both said, in effect, *the viewer's parser reads these
bytes correctly and then throws away the fact that it had to*:

- **ISO 19005-2 §6.1.4, ISO 19005-4 §6.1.4** — the `xref` keyword and the cross-reference
  subsection header shall be separated by a single end-of-line marker. `pdf_syntax` kept the merged
  table it had built and nothing about where it read each section from, so nothing in the tree could
  say where a section's `xref` keyword stood.
- **ISO 19005-2 §6.1.6, ISO 19005-4 §6.1.5** — a hexadecimal string shall always have an even number
  of digits. ISO 32000-2 §7.3.4.3 tells a reader to complete an odd one with a zero, and the lexer
  does; the string it hands back is byte for byte the string a well-written one would have produced,
  so the fault is gone before a validator sees the object.

A third question turned out to have the same shape once it was read. ISO 19005-2 §6.1.3 requires the
file trailer dictionary to contain `ID`, and `XrefTable::trailer` is the **merge** of every section
on the `/Prev` chain. ISO 32000-1:2008 §7.5.6 and ISO 32000-2 §7.5.6 both require each appended
trailer to restate its predecessor's entries itself, so the merge answers a different question from
the one the clause asks — and it answers it wrongly in exactly one direction, passing a file whose
newest trailer states nothing.

## What was added, and what it costs

Three things in `pdf-syntax`, and the rule that governed all three is the owner's: *if exposing what
the validator needs costs the viewer nothing, do it; if it costs something real, measure it first.*

1. **`pdf_syntax::xref::sections(file, limits) -> Vec<SectionRecord>`** — the `startxref` and `/Prev`
   chain walked again, reporting each section's byte offset, whether it is §7.5.4's classic table,
   and the trailer **that section** states before any merge. Nothing on the opening path calls it.
   The walk itself is the one `read` already does, reached through one new `Option<&mut Vec<_>>`
   parameter on the private `read_from_startxref`: a document that never asks pays one `Option` test
   per section — one to three of them in a real file — and no allocation.
2. **A `classic` flag on the private `Section`**, so that a caller can tell §7.5.4's table from
   §7.5.8's cross-reference stream. A rule about the bytes around `xref` has no subject in a section
   that states no such keyword, and §7.5.8.1 forbids one there outright.
3. **`Lexer::hexadecimal_strings() -> HexadecimalStrings`** — how many §7.3.4.3 strings the lexer has
   read, how many stated an odd number of digits, how many held a byte that is neither a digit nor
   white space, and where the first of each stood. Nothing is counted per byte of a well-formed
   string: the digit count is `out.len()` and the parity is the `pending` half-byte, both already
   held, and the stray test runs only on a byte the hexadecimal reading has already rejected.

**The measurement, because startup is a first-class requirement.**
`cargo run --release -p pdf-syntax --example callgrind_open`, ten opens of ISO 32000-2's own
specification — 19 MB, 101 318 objects, two cross-reference sections — under callgrind, which counts
instructions and is therefore unaffected by load or frequency:

| | I refs, ten opens |
|---|---|
| before | 613 706 284 |
| after | 614 074 711 |

**36 843 instructions per open, +0.060%**, deterministic across repeated runs. That is the parameter,
the flag and the five fields on `Lexer`, and it is small enough that the honest thing to say is that
it is not zero rather than that it is free. No allocation was added to the open path and no public
signature changed; `SectionRecord` and `HexadecimalStrings` are new public types, and `read` and
`Lexer::new` are called exactly as before.

## What this bought, and what it did not

Nine corpus documents this crate had been missing, across three clauses and both parts, with `over`
— a conforming document failed — staying at zero on all six targets. Two rows moved off
`Check::Unchecked`; a third row was added for the base-standard rule beside one of them, cited at
§5.1 rather than at ISO 19005's own clause, because §6.1.6 and §6.1.5 state how many digits a
hexadecimal string has and say nothing about what a digit is.

What it did not buy is a cheap check. The two hexadecimal rules are now the two dearest predicates in
the crate — 266 ms and 146 ms on that same document, against a 4.1 s report — because each walks the
document's syntax itself: the file's own objects, §7.5.7's object streams, and every page's content.
The second is cheaper only because decoded streams are memoised; the lexing is done twice, and the
place that would fix it is a field on `crate::Examination`, where a report's shared work already
lives. Reading the file whole instead of a window per object was tried and refuted — 274 ms became
266 ms for 19 MB retained — so the cost is the lexing rather than the reading, and the note above
`hexadecimal_spans` records both numbers so that the next reader does not repeat the experiment.

## The trap this nearly walked into

Lexing a content stream from end to end **invents hexadecimal strings out of inline image data**.
§8.9.7's `BI … ID … EI` puts arbitrary bytes inside a content stream, and compressed samples hold
angle brackets like they hold anything else; five conforming corpus documents were failed by the
stray-byte rule before `pdf_model::inline_image::scan` was used to skip past `EI`. The direction of
error the crate states — under-report rather than mis-report — is not a slogan here: a validator
telling a user their conforming file does not conform is the one outcome that makes it useless.
