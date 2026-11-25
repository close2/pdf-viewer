# 3677 — A number in exponential form is written as the value this reader read it as

Session 2244. Status: **accepted and built**; extends ADR 2638 (`archive/as_drawn.rs`). Context: ADR
3662's census, which left `syntax/number-with-an-exponent` refused at the fence (`sci-notation`,
`367594`); ADR 0733, the lexer's reading. Clauses: ISO 32000-2 §7.3.3; ISO 19005-2 and ISO 19005-4
section 5.1.

## Context

§7.3.3 ends: "A PDF writer shall not use the PostScript language syntax for numbers with
non-decimal radices (such as 16#FFFE) or in exponential format (such as 6.02E23)." The `shall not`
is the writer's; nothing is stated for a reader. This reader reads such a run as its value and logs
the class, so section 5.1 fails, and ADR 2638's run rewrite — the only write that reaches the run —
was made only where section 6.2.2's operator row asked for it.

## Decision

1. Wherever `as_drawn`'s write is made, a real whose run holds an `e` or `E` is written as the
   decimal `pdf_syntax::write::object` writes for the value read: the shortest that reads back as
   the same value, never with an exponent. Nothing else in the stream changes.
2. The write is asked for wherever section 5.1 fails on this class under a profile granting
   `content-as-drawn`, as for ADR 3558's operators; the class is answered where every stream of its
   page or form is rewritten, and the output's own check holds the write to having reached each.
3. The loss is `content-as-drawn`: the value is the one the clause's own example glosses ("6.02E23"),
   but the producer's spelling is gone. No proof by drawing is needed — the token is the same number
   — and none is made.
4. `syntax/number-salvaged` keeps ADR 2638's answer; its census carriers (`455746`, `498264`) hold
   other classes and operators no write answers (ADR 3680).

## Consequences

`sci-notation` passes section 5.1 and is refused by section 6.2.2's operator row (`ETBT`, an
undefined operator); `367594` passes this class and stops at `syntax/string-unterminated`.
