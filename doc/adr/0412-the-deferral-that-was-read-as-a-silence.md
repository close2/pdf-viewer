# ADR 0412 — The deferral that was read as a silence

Status: accepted, 2026-08-18. Session 577. Reverses ADR 0371 decision (b) and the answer sent to
the quorra team as `doc/QUORRA_FEEDBACK.md` §26.3(b): a type 4 program in which `ge`, `gt`, `lt` or
`le` **provably** reads a boolean operand is refused at parse time, and the caller reports it.
Adds `pdf_model::function::ordering_reaches_a_boolean`, a compile-time walk over types where
`fold_constants` walks over values. Amends §7.10.5.2's ledger row. Closes item 1 of
`doc/todo/54-what-quorras-answer-asked.md`.

Measured population of provable cases over 1 251 documents: **zero**. No page moves.

## 1. The question, and where it came from

The quorra team's ADR 0053 §3.2 asked what this tree's contract is when one of the four ordering
operators compares booleans, where PostScript raises `typecheck`. The five-hundred-and-thirty-sixth
session answered *convert*, wrote the answer into ADR 0371 as decision (b), and sent it upstream
with its ground. `doc/todo/54` carried the question back with the code witness beside it:
`function.rs`'s four arms compare `Value::as_f64`, and `as_f64` maps `Boolean(value)` to 1 or 0, so
`true 0 gt` is true and nothing is said.

The todo file named three outcomes — the clause states the restriction, the clause states nothing,
or the clause states the coercion — and they are three different pieces of work. This round's job
was to find out which, and it is the first.

## 2. What §7.10.5 says

§7.10.5.2 states no semantics of its own. It hands them over, and the sentence is a `shall`:

> The PostScript Language Reference, Third Edition shall define the semantics of these operators
> and all other syntax rules of the PostScript language. Although the semantics are those of the
> corresponding PostScript language operators, a full PostScript language compatible interpreter is
> not required.

That document is in clause 2, **Normative references**, whose own preamble says what being there
means:

> The following documents are referred to in the text in such a way that some or all of their
> content constitutes requirements of this document.

So the operand types of `ge`, `gt`, `lt` and `le` are a requirement of ISO 32000-2, stated by
reference. `doc/habits.md`'s *Reading the specification* has the rule this is an instance of —
"where the standard defers to another document, the deferral is a citation" — and it was written
about §9.7.5.3's hand-off of a CMap's syntax to a technical note.

Annex B corroborates, and its corroboration is worth more than a summary usually is because of
what it distinguishes. §B.3's operand column gives the relational and boolean operators **three**
different classes in one table:

| operands | operators |
|---|---|
| `any 1 any 2` | `eq`, `ne` |
| `num 1 num 2` | `gt`, `ge`, `lt`, `le` |
| `bool 1 \| int 1 bool 2 \| int 2` | `and`, `or`, `xor` (and `not`, one operand) |

Three classes drawn on purpose, in one table, by a standard that could have written `any` in all of
them. There is no silence here to fill. Annex B is informative and says so on its title line, so it
is not what makes this a requirement — the deferral is — but it is what makes the reading
unambiguous without holding PLRM3, which this project does not (§7.10.5.2's ledger row has said so
since the five-hundred-and-thirty-fourth session).

`cargo run --release -p spec-errata -- emit doc/*.pdf` was run first, as `doc/todo/02` §4 requires
of a round implementing a clause. Errata Collection 3 carries four annotations in the §7.10.5
family — a digit in §7.10.5.1, `If`→`if` and `07hD`→`7Dh` in §7.10.5.3 — and not one of them
touches §7.10.5.2, Table 42 or Annex B's operand columns.

**So it is outcome one: the clause states the type restriction, and this tree departed from it in
silence.** ADR 0371's decision (b) was not wrong about §B.3's line; it was wrong one step earlier,
in reading "§7.10.5.1's subset admits no value meaning *error*" — a true fact about the
*evaluator* — as though it were a fact about the *clause*.

## 3. Refuse, rather than report and draw

Trap 5 says make it loud and offers two shapes: refuse, or draw and say so. Three things decide it
here, and the order matters.

**The additive-or-substitutive test.** A comparison's answer is not an extra mark on the page; it
decides a branch, and the branch decides a colour over the whole region the program was written to
distinguish. Substitutive. This is the same test that refuses a type 0 function whose sample array
is short (`Function::sample_extent`, ADR 0356), and for the same reason.

**ADR 0339's test, run rather than quoted.** `EMPTY_STACK` chose a *value* for §7.10.5.3's
underflow on an explicit measurement — "refusing the program instead would refuse a document that
draws", with `pi_seven_segment.pdf` as the witness. That is the right test and this round ran it for
this operand class instead of assuming the answer carried across. It does not: §4 below is zero.
Same test, different measurement, opposite answer.

**Where the refusal can be raised without a per-pixel cost.** ADR 0339's second objection — a report
per evaluation is one per device pixel of a shading — is an objection to a *runtime* answer.
Deciding it at compile time answers both objections at once: it is once per `Function::parse`, on
the program the file states, and it leaves through `FunctionError`, which every caller of
`Function::parse` already turns into a report. Nothing was plumbed to make that work; the channel
was there.

## 4. The population, measured before the fix was priced

`doc/todo/54` asked for this and `crates/pdf-model/examples/type4_type_census.rs` already answers
most of it, which is `CLAUDE.md`'s rule about commands and facts working as intended. Over
`doc/pdf.js/test/pdfs`, the four corpora under `doc/corpora/` and `doc/corpora-own/` — 1 251
documents, 91 of them naming `/FunctionType` in their own bytes, 21 carrying a type 4 function, 44
type 4 functions between them:

| shape | functions |
|---|---:|
| **containing** an ordering operator and a boolean source from elsewhere | 2 |
| **provably** applying an ordering operator to a boolean | **0** |

**The two counts are the whole point, and trap 11 is why.** Containment is what a `grep` can see and
it is an over-estimate: both witnesses are the project owner's own hand-written files, and reading
their programs says why. `type4_pi.pdf`'s ten rectangle tests are `dup 85 ge exch 95 le and` —
`ge` and `le` each see a coordinate, and the `and` above them eats the two booleans without either
reaching another comparison. `pi_seven_segment.pdf`'s eight segment tests have the same shape, and
its one `4 index 0 eq and` puts a boolean under `eq`, which §B.3 types `any 1 any 2` and which is
therefore not this at all.

The census now counts the provable shape directly, by parsing each program twice: a program the tree
refuses while its rewritten untyped arm — where `true` is `1.0` and every comparison is followed by
`cvr`, so no boolean exists in it — still parses is exactly one of these. `44 of 44 compared, 0
refused` after the change, which is the number this decision rests on.

**A departure no document can reach is still a departure**, which is why it is fixed rather than
recorded; what the measurement decides is that refusing costs nothing today rather than whether the
reading is right.

## 5. What the walk is, and which way it errs

`ordering_reaches_a_boolean` is `fold_constants`' walk with a different lattice. Same forward pass
over the compiled instruction list — which has no backward jumps, so one pass in index order is the
whole dataflow — same absolute-depth model, same `promise`d depth at a jump target, same
abandonment where the depth stops being modellable. Where `fold_constants` holds `Option<Value>`,
this holds a three-rung `Slot`: the value, or only its `Sort` (§7.10.5.1's three types), or nothing.

The third rung is why it is a separate walk rather than a use of the existing one. A program whose
comparison reads a *computed* boolean — `x 0.5 gt 0 gt` — has no literal under the second `gt`, and
`Option<Value>` can only say it does not know.

`apply_sorts` mirrors `apply_operator` arm for arm, with the same `pop`, `unary` and `binary`
helpers so that the depth it leaves is the depth the evaluator leaves, and with no wildcard arm, so
a Table 42 operator added to `Operator` stops the build rather than reaching the walk as nothing.
That is `device_step`'s discipline, one function over.

**It under-approximates on purpose.** Three places give up precision and each is a decision:

- **a join forgets everything**, rather than merging the arms' types — so
  `{ 0.5 gt { true } { false } ifelse 0 gt }` is *not* proven, and a test says so on the record;
- **`add`, `sub`, `mul` and the six one-operand arithmetic operators answer `Unknown`** unless an
  operand is already a real, because `arithmetic2`'s `checked_` arm falls back to a real on
  overflow and whether it does is a fact about values;
- **a `copy`, `index` or `roll` whose count is not a known value abandons the program**, and a
  `roll` whose *shift* is unknown forgets its window, because a permutation the walk cannot follow
  is a slot read in the wrong place.

The direction is the one with no cost: a departure the walk misses is drawn exactly as it was drawn
before, where the other direction would refuse a document over an analysis this project wrote. The
conversion policy in `Value`'s doc comment therefore still stands for everything the walk admits,
and `an_ordering_operator_reads_a_boolean_as_a_number_rather_than_refusing_it` is what pins it.

**And the order against the fold is load-bearing.** `true 0 gt` is a closed island, so
`fold_constants` replaces the whole departure with the one value this reading says it does not have.
The walk runs first.

## 6. What this round did not do

Three neighbours of this defect are the same shape and are *not* touched, deliberately, because each
would need its own measurement and its own argument:

- §7.10.5.3's underflow, which ADR 0339 settled with `EMPTY_STACK` and a witness that draws;
- §7.10.5.3's output-count error, answered by padding with zeros;
- `div`, `idiv` and `mod` by zero and `ln`, `log`, `sqrt` outside their domains, which ADR 0369
  settled as choices where the deferral cannot be quoted.

The difference is that this one has a clause that states the restriction and a population of zero.
Those three have neither, and a round that swept them together would be applying this round's
conclusion rather than deriving theirs.

## 7. What it cost, and what was checked

No page moves. Every gate `doc/todo/02` §2 lists was run: fmt, clippy over the workspace, 2 147
workspace tests, the doctests, the corpus gate (974 documents, 66 incomplete), the oracle (1 794
pages), both text-extraction gates, dates, XMP, JPEG 2000, the quorra corpus gate (957 pages, 932
agree) and the conformance gate. Every ratchet held.

The one thing worth naming as a risk is the walk's own correctness: an analysis that *wrongly*
proves a boolean refuses a document that draws. It is mitigated three ways — the under-approximating
lattice above, a test naming every conforming shape the corpus's own functions are made of, and a
census that would have printed any refusal by path and object number.
