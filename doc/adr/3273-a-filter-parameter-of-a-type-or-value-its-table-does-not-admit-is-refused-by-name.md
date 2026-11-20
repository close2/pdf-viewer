# 3273 — A filter parameter of a type or a value its table does not admit is refused by name

Status: accepted. Session 2143. Context: ISO 32000-2 §7.4.4.3, §7.4.4.4, §7.4.6, Tables 6, 8, 10 and 11; audit 2142
(`doc/reviews/2142-specification-audit-7-4.md`) findings B03, B04 and B05; ADR 2729; trap 5.

## The defect

Every Table 8 entry was read with `as_integer` and its default put in place of anything else, so `/Predictor 12.0` was
read as absent and the PNG rows came back with their tag bytes as the decoded data; a negative `/Colors`,
`/BitsPerComponent` or `/Columns` became its default, and `/EarlyChange 2` became 1. Table 11's booleans and `/K` in
`pdf-model`'s `ccitt.rs` were read the same way. The PNG predictor un-predicted at a `/BitsPerComponent` Table 8 does not
list (the TIFF one already refused it, ADR 2729), and a predictor ran after any stage whose parameters named one, so
`RunLengthDecode`'s `1 2 3` came out `1 3 6`.

## The question, and the two answers

A present entry the table does not admit is a statement no reader can run. It can be **refused by name** — the stage
produces nothing and says which entry — or **reported as damage** with the data withheld. The second is not a thing
`Decoded` can say: damage there means bytes came out and stop short, and here no byte is the file's. So it is the first.

## Decision

- `FilterRefusal::Parameter { key }` is the refusal. `pdf_syntax::filter::prediction` reads Table 8 once for the whole
  decode and the window: an absent or `null` entry is the default; an integer is checked against the table — `/Predictor`
  one of Table 10's 1, 2, 10 to 15, `/Colors` and `/Columns` at least one, `/BitsPerComponent` one of 1, 2, 4, 8, 16
  ("May be used only if Predictor is greater than 1", so read only then) — and anything else, a reference left
  unresolved included, is refused. `/EarlyChange` is 0 or 1 or refused.
- A predictor belongs to `LZWDecode` and `FlateDecode` alone (Table 8's heading, §7.4.4.4's first sentence); a
  `/Predictor` in any other stage's parameters is read by nothing.
- `apply_predictor` refuses an unlisted depth for both function groups, so the public function holds the rule too.
- Table 11's entries are read by the same rule in `ccitt_bilevel` and refused as `ImageError::Malformed` naming the
  key: a boolean that is not a boolean, a `/K`, `/Columns`, `/Rows` or `/DamagedRowsBeforeError` that is not an
  integer, and a negative `/Rows` or `/DamagedRowsBeforeError`. `/K` travels as its sign, because the filter "shall not
  distinguish between different positive K values" and a positive `/K` past `i32` was read as 0.

## Consequences

The audit tests for B03, B04 and B05 pass. A window (`Document::pumping`) declines a stage whose Table 8 parameters are
refused, so the whole route's refusal is what a reader meets. The corpus walk before and after is in the round's record.
