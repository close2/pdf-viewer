# 577 — The deferral that was read as a silence

**The finding**: §7.10.5.2 states no operator semantics of its own — it hands them to a document
ISO 32000-2's clause 2 makes normative, so the operand types of `ge`, `gt`, `lt` and `le` are a
requirement of the standard rather than another language's habit. This tree read that absence of
local text as a silence for forty-one sessions, converted a boolean operand into the 1 or 0 it
stands for, and said nothing. It refuses now, where a compile-time walk can prove one reaches the
operand — and the population of provable cases across 1 251 documents is zero, so no page moves.

Date: 2026-08-18. Argued in
[ADR 0412](../adr/0412-the-deferral-that-was-read-as-a-silence.md).

Touched: `crates/pdf-model/src/function.rs` (`ordering_reaches_a_boolean`, `Slot`, `Sort`,
`apply_sorts`, `ordering_operands`, a generic `operand_demand`, the refusal in `parse_postscript`,
four corrected doc comments and four tests), `crates/pdf-model/examples/type4_type_census.rs` (the
provable shape beside the containment one), `doc/conformance/ledger.toml` (§7.10.5.2),
`doc/QUORRA_FEEDBACK.md` (§26.3(b) withdrawn in place),
`doc/adr/0371-…` (decision (b) marked superseded), `doc/todo/54-…` (item 1 closed),
`doc/adr/0412-…`, this file.

## Where the question came from

Not from a gate and not from the corpus. The quorra team, building a device-side evaluator against
this tree's, asked in their ADR 0053 §3.2 what our contract is when an ordering operator compares
booleans. The five-hundred-and-thirty-sixth session answered *convert*, wrote it into ADR 0371 as
decision (b), and sent the ground upstream in `doc/QUORRA_FEEDBACK.md` §26.3(b) — with the sentence
"**Do not refuse.**" at the top of it. `doc/todo/54` carried the question back with the two code
lines beside it.

Both of those answers were built on one sentence, and the sentence is true: §7.10.5.1's subset has
no value that means *error*. What was wrong is what it was taken to license. It is a fact about the
evaluator — a value has to come back from `apply_operator` — and it was read as a fact about the
clause, as though the standard had left the operand types open. It had not; it had deferred them,
and a deferral is a citation.

## The reading

§7.10.5.2, verbatim:

> The PostScript Language Reference, Third Edition shall define the semantics of these operators
> and all other syntax rules of the PostScript language.

Clause 2's own preamble, which is what makes that sentence bite:

> The following documents are referred to in the text in such a way that some or all of their
> content constitutes requirements of this document.

And Annex B, informative, corroborating in the sharpest way an informative annex can — by
distinguishing. §B.3 gives the relational and boolean operators three operand classes in one table:
`any 1 any 2` for `eq` and `ne`, `num 1 num 2` for the four ordering operators, `bool | int` for
`and`, `or`, `xor` and `not`. A standard that meant "any" in all three would have written it once.

`spec-errata emit` first, as `doc/todo/02` §4 requires: four annotations in the §7.10.5 family, none
of them on §7.10.5.2, Table 42 or Annex B's operand columns.

## Two counts, and why the round has both

`doc/todo/54` asked for the population before the fix was priced, and the census that answers it
already existed — `type4_type_census`, written for ADR 0371. Over 1 251 documents it finds 44 type 4
functions. Two of them *contain* both an ordering operator and a boolean source from elsewhere.
**None** of them provably feeds one to the other.

Trap 11 is the whole reason to print the difference rather than the count. Both containment
witnesses are the project owner's own hand-written files, and their programs say why: `type4_pi.pdf`
tests ten rectangles with `dup 85 ge exch 95 le and`, where each comparison sees a coordinate and
the `and` above consumes both booleans; `pi_seven_segment.pdf` does the same eight times, and its one
`4 index 0 eq and` puts a boolean under `eq`, which the clause types `any 1 any 2` and which is not
this defect at all. A containment grep would have reported two and been wrong twice.

The census counts the provable shape directly now, and it is the instrument that would have named
any refusal by path and object number. It printed none.

## Why refuse rather than report and draw

`EMPTY_STACK`'s doc comment already carries the right test for this family and ADR 0339 ran it for
§7.10.5.3's underflow: refusing would refuse a document that draws, and `pi_seven_segment.pdf` is
the witness. This round ran the same test for this operand class instead of assuming the answer
carried over, and it comes out the other way — nothing draws that this refuses. Same test, different
measurement, opposite answer, and that pair is more of what the round is worth than the code is.

The other half of ADR 0339's objection — a report per evaluation is one per device pixel of a
shading — is an objection to a *runtime* answer. Deciding it once per `Function::parse` answers it,
and `FunctionError` is a channel every caller already reports through, so nothing had to be plumbed.

## The walk, and the direction it is wrong in

`ordering_reaches_a_boolean` is `fold_constants`' own forward pass with a three-rung lattice instead
of two: a slot's value, or only which of §7.10.5.1's three types it is, or nothing. The third rung
is the reason it is a separate walk — a comparison reading a *computed* boolean has no literal under
it, and `Option<Value>` can only say it does not know.

It under-approximates at three places, each a decision on the record and one of them a test: a join
forgets its stack rather than merging the arms, the arithmetic whose result type depends on overflow
answers `Unknown`, and a counting operator whose count is not a known value abandons the program.
The cost of being wrong is therefore a departure left where it was, never a document refused over an
analysis this project wrote.

## Gates

Everything `doc/todo/02` §2 lists, all green: fmt, clippy across the workspace, 2 147 nextest tests,
the doctests, the corpus gate at 974 documents and 66 incomplete, the oracle at 1 794 pages, both
text-extraction gates, dates, XMP, JPEG 2000, the quorra corpus gate at 957 pages with 932 agreeing,
and the conformance gate. Two conformance findings were mine and were fixed before the run that
counted: a `§` in front of another project's ADR section number, and a blockquote whose ellipsis I
had written as an em-dash.
