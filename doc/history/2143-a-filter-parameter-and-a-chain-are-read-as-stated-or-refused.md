# Round 2143 — a filter parameter and a filter chain are read as stated, or refused by name

A fix round in the `audit` worktree for audit 2142's groups C and A (§7.4): B03, B04, B05, B01
and D01. Groups B, D and E are later rounds'.

## What changed

- ADR 3273: a Table 8 or Table 11 entry present with a type or value its table does not admit is
  refused by name (`FilterRefusal::Parameter { key }`; `ImageError::Malformed` naming the key for
  CCITT), never read as its default. `filter::prediction` reads Table 8 once for the whole decode
  and the window; a predictor belongs to `LZWDecode` and `FlateDecode` alone (B05); both
  predictor groups refuse a depth Table 8 does not list (B04); `/EarlyChange` is 0 or 1; `/K`
  travels as its sign.
- ADR 3274: `Document::stated_chain` / `chain_of` is the one reading of `/Filter` and
  `/DecodeParms`, used by the whole route, the window, images, the inline-image extent, the
  writer, the crypt-filter lookup and the cross-reference stream; a malformed chain is
  `StreamRefusal::Chain(MalformedChain)` (B01). One dictionary over a longer chain is the one
  parameter-taking filter's, and refused where two or more take parameters (D01).
- The four audit tests pass; new tests in `filters.rs` (Table 8, one dictionary) and `ccitt_bound.rs`.

## Ledger

7.4.1, 7.4.4.3, 7.4.4.4 and 7.4.6 stay `implemented` (the findings that contradicted them are
fixed), each with the new tests and a sentence. 7.3.8, 7.3.8.2, 7.4.2, 7.4.3 and 7.4.10 wait for
groups B and D.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets` over `-p pdf-syntax -p pdf-model -p
pdf-render` exit 0 on the third run (type complexity, doc backticks). `pdf-sandbox --bins` exit 0.
nextest `--lib --bins --tests` of the three (`xvfb-run -a`): 3627 run, 3624 passed, 3 failed — the
manifest's B02, B06, B07 ("failed 3, expected 3"). rustdoc exit 0; fuzz `check --bins` exit 0;
conformance exit 0 on the second run (a doc path named the removed `decode_parms`; this paragraph).
The `corpus` walk fails identically before (HEAD, from a snapshot) and after: 66 incomplete
against a ceiling of 65, `issue17929.pdf` joined (a font with no `/FontDescriptor`, not a filter);
every report line equal.

**Left.** Groups B (B02), D (B06, B07, D02), E (D03); a `/DecodeParms` array shorter than its
chain gives the stages past its end their defaults (ADR 3274 records it as a choice).

**Habit.** A chain has a shape as well as names: a `filter_map` over what the file states decodes
something it did not state, exactly as an `unwrap_or(<default>)` over a parameter does.
