# 534 — The two operators a deferred clause decides, and the tie nobody had chosen

2026-08-15. One round, taking the two defects the quorra team left in
`doc/QUORRA_FEEDBACK.md` section 25.6. ADR 0369 is the decision and the argument; this is the
record.

## What was taken

Both defects, plus the audit of the whole of Table 42 that a set wrong twice deserves, plus a
census that says what either cost in the wild.

- **`Operator::Round`** was `f32::round`, half away from zero. It is `round_to_greater` now: a tie
  goes to the greater integer, `-6.5` to `-6`.
- **`Operator::Eq`/`Ne`** compared within `f32::EPSILON`. They are `==` and `!=` now.

## What the round is actually about, which is neither of those

§7.10.5.2 states **no semantics at all** — it defers them to the PostScript Language Reference,
Third Edition — and nothing in this tree had written that down. The ledger row described Table
42's contents and the stack bound; it did not say the clause defines nothing itself. So the
operators had never been read against anything.

This tree does not hold PLRM3, and nothing in this round quotes it. What went in instead is the
extension of principle 5 that the case needs: where the standard defines something *elsewhere* and
the elsewhere is out of reach, the reading is written down **as a choice with its ground**, exactly
as it is where the standard defines nothing. `apply_operator`'s doc comment now says so, with
§7.10.5.2's deferral quoted verbatim and Annex B — informative, one line per operator — quoted
where its line settles an arm.

## The audit

Nine tests added, no third defect found, three answers reclassified as choices and marked at the
site: `bitshift`'s integer width on a right shift of a negative value; the error `div`, `idiv`,
`mod`, `ln`, `log` and `sqrt` cannot express in a subset whose only values are numbers and
booleans; and `not`, which Annex B makes two operators wearing one name and which a compiled
literal carrying no type can only be one of. That last one is quorra's contract question 3 and
`doc/QUORRA_FEEDBACK.md` section 25.5 already assigned it to the type-tag work; the test pins
today's answer rather than pretending it is right.

Settled by Annex B and correct as they stood, each now with its line and a test: `atan`'s two
operands and its whole circle, `idiv` and `mod` as one truncating convention, `cvi` and `truncate`,
`and`/`or`/`xor` needing no discrimination because a boolean here is 1 or 0, `roll`'s direction.
And one containment worth a test: `exp` is the only arm that produces a `NaN` deliberately, and
§7.10.5.3's required `/Range` is what stops it reaching a caller.

## The census, and what it found

`crates/pdf-model/examples/type4_operator_census.rs`, new, modelled on ADR 0361's. Over 67 462
paths — pdf.js, the four corpora, the owner's two files, SafeDocs and openpreserve — it found
7 353 type 4 functions in 2 099 documents and reported, per Table 42 operator, how many programs
contain it, and for `round`, `eq` and `ne` whether the value moved.

**`round`, `ne`, `bitshift` and `not` are reached by none of the 7 353.** `eq` is reached by
exactly one: `doc/corpora-own/pi_seven_segment.pdf`, the project owner's hand-written file, and
its value does not move at any sample of a nine-point grid over its `/Domain`.

So sixty-seven thousand files could not have found either defect, and one person reading the
source against the clause found both. Trap 8's cleanest instance in the tree.

The "before" arm is a source rewrite and is **exact** rather than approximate — the old `eq` is
`sub abs <eps> lt` to the bit, and the old `round` is the new one minus the one condition where
they part — and the instrument was validated on seven synthetic files before it was believed.

## What moved: nothing, and it is proven

- `display_list_digest` over all 974 pdf.js documents, before and after in one working copy with
  the patch the only variable: `diff` empty.
- Every gate green, every number identical to the last recorded baseline: corpus 974 documents /
  64 incomplete; oracle 1 794 pages, 906 agrees / 67 contradicted / 786 ambiguous; quorra scale 1
  931/23/2/18 and the `gpu` lane at 4× 937/9/5/23; text extraction 99.2% and 98.26%; conformance,
  dates, xmp, doctests all pass; nextest 1961/1961.
- The one witness in the population rendered at 2× before and after: byte-identical, and looked
  at — the seven-segment display still reads 3.141.

`doc/todo/00`'s step 7 ink sweep is not owed: it is for a round that changes what gets drawn, and
this one is measured not to have.

## Written back

`doc/QUORRA_FEEDBACK.md` section 25.6.1 answers their section 6 where they left it, since they
asked: both confirmed, both fixed, the audit, the census, and one correction they can use — WGSL's
half-to-even and this tree's half-toward-greater agree at `-6.5` and **part at `2.5`**, so a
device-side `round` needs refusing or correcting the way their transcendentals do.

## Files

`crates/pdf-model/src/function.rs`, `crates/pdf-model/examples/type4_operator_census.rs` (new),
`doc/conformance/ledger.toml` (§7.10.5, §7.10.5.2), `doc/adr/0369-…`, `doc/QUORRA_FEEDBACK.md`,
this file.
