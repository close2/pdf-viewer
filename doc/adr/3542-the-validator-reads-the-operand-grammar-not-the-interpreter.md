# 3542 — The validator reads §7.8.2's operand grammar off the tokens, and does not run the interpreter

Session 2210. Status: **accepted**. Context: ADR 3477, ADR 3541; review 2194's B08 ("the round
decides by the cost of the second walk"); ISO 32000-2 §7.8.2, §8.4.2, §8.6.8, §9.4.1.

## Context

B08's five repairs — an operand of the wrong type, an operator short of its operands, a `Q` with
nothing saved, a text operator outside a text object, colour operands that do not fit the space —
are made by the interpreter, which `pdf-archive` never runs: its survey walks the tokens. Running the
interpreter per page and per form in `Examination::read_whole` would decode every image and load
every font a second time beside that walk.

## Decision

- **No interpreter in the validator.** Each of the five is a function of the token sequence, the
  `q`/`Q` and `BT`/`ET` nesting and the component count of the space a `cs` selected, so
  `pdf_model::content::grammar` states them once: `signature(word)` gives each Table 50 operator its
  operand list and role, and `Grammar` is fed operands and operators and answers the class.
- **One table, held to the interpreter's.** `operand_counts_are_the_grammars` holds every
  `Operator::operand_count` to its signature's length, so the two readers cannot drift.
- **The survey feeds it** in the walk it already makes, counts a resource space's components with
  `grammar::components` (§8.6.4 to §8.6.6, Table 61), and hands what it finds to the stream's reader
  (`note_repair`), which records it at the stream's content site with the lexical classes.
- The interpreter logs the same classes where it makes the repairs, for the viewer; the log
  deduplicates class and site, so a page both walked and drawn is logged once.

## Consequences

- The validator's cost is one small fixed array per stream and a comparison per operator.
- The grammar answers only what the tokens decide: a leftover operand in front of an operator's
  own (T05) and a `Tr` written as a real are not its questions.
