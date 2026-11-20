# 3267 — The operand cap an array left open by damage reached is the stream's own fault

Session 2141, an archive round in the `gwg` worktree. Status: **accepted** and built. Context: ADRs
2516, 2618, 2759, 2760; `doc/pdfa-blocking-issues.md` R08 and R03 (`427669`);
`crates/pdf-transform/src/archive/fonts/photographed.rs` (`an_array_left_open_by_damage`).
Clauses: ISO 32000-2 §7.3.6, §7.8.2; ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1.

## Context

`427669` stopped at "a bound this reader sets stopped its drawing". Printed: page 17's
`LimitReached { limit: "MAX_OPERANDS" }` (`pdf-model`'s cap, 8 192). The page's third `/Contents`
stream fails its own Adler-32 (`Damaged { damage: CheckValue }`), and its decoded bytes end with
four `[` left open (`… 72 04w [7.92 …`, a dash array the damage broke). The interpreter keeps an
array's elements as operands until its `]`, so every token of the five streams after it piled up
as operands, each keyword among them reported as a keyword inside an array (§7.3.6), until the
cap — where the interpreter also stops believing the array (`run/steps.rs`).

## Is the bound the right one for a valid page?

For a page whose bytes decode whole, the cap is reached only by stating 8 192 objects before one
operator (arrays are kept flattened). That is a bound principle 3 asks for; whether a valid `TJ`
of that length should be read further is the leftovers lane's question, not this one: no corpus
page reached it whole.

## Decision

Under the wider gate, the operand cap does not hold a page whose reports also name a part of its
`/Contents` that failed its own decode and a keyword inside an array: the operands behind the cap
are the open array's, the stream's own fault, baked in as the window drew it (ADR 2618). Each
condition alone, or the cap alone, keeps the bound's refusal. The test is per page because the
report names no stream; `pdf-model` is left unchanged.

## Measured

`427669` converts at part 4 under `as-if-printed` (2b at section 6.1.13, ADR 2498); veraPDF 1.30.3
passes it; pages 17 and 18 are photographs whose 60 dpi drawing matches the source's but for
resampling. Test: `the_operand_cap_an_array_left_open_by_damage_reached_is_the_stream_s_own_fault`.
