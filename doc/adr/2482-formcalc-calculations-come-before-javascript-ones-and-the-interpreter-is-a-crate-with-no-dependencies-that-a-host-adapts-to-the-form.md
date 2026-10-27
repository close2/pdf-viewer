# 2482 — FormCalc calculations come before JavaScript ones, and the interpreter is a crate with no dependencies that a host adapts to the form

Session 1824, a clause round in batch 123. Status: accepted. ADR 2483 is unused.

## Context

RFC 0011 section 5 owes §K.2's calculations and events in two steps: step 5, JavaScript through
`pdf-script`, and step 6, FormCalc, the language a script uses where it states no `contentType`
(XFA 3.3 page 412; the grammar is pages 1049–1080 and the built-in functions pages 1081–1149). The
RFC put 5 before 6 because its section 4 census, over `doc/pdf.js` and `doc/corpora`, found four
hybrid forms and no calculation among them. The brief of this round asked for the census to be
taken again before the order was fixed, and for the smallest executable subset to be chosen from
what the forms carry.

**The census** (`crates/pdf-formcalc/examples/xfa_script_census.rs`) walked those two trees and
the crawl cache: 90 319 documents opened, 423 unopenable, 21 over the 128 MiB bound, none panicking;
149 dynamic XFA forms, 122 hybrids, 17 whose resource does not read. The premise of RFC 0011
section 4, that the hybrids' templates hold almost no behaviour, is a statement about 1 479 files
and does not hold of the larger population: 11 hybrids carry a `calculate` script.

| in the hybrids | live | commented out or empty |
|---|---|---|
| `calculate`, FormCalc | 24 scripts in 5 documents | none |
| `calculate`, JavaScript | 36 scripts in 5 documents | 62 |

The 24 FormCalc calculations use arithmetic `+ - * /` and parentheses over dotted field names,
`$` (152 uses), assignment to `$`, `var`, `if … then … endif`, and four built-ins: `Replace`
(76 calls, one script of 76 lines that rewrites a state code in place), `Sum` (13), `Concat` (3),
`HasValue` (1). Every live JavaScript calculation reads `rawValue` or calls `xfa.resolveNode`, the
SOM host object that step 5 builds and `pdf-script`'s surface does not have. The hybrids' events are
mostly JavaScript (about 4 300 live, against about 180 in FormCalc), which is why the events stay
owed with step 5.

## Decision

1. **FormCalc calculations first.** Step 6's subset is built before step 5, for calculations. The
   whole FormCalc `calculate` population of the census is within reach of a small evaluator, and
   the JavaScript population needs an object that has to be designed first. Both need the same
   page 114 scope search, which the hook below supplies once.
2. **The interpreter is `pdf-formcalc`**, a leaf crate with no dependencies and
   `#![forbid(unsafe_code)]`. It lexes the whole lexical grammar (pages 1054–1058), parses the
   whole syntactic grammar (pages 1058–1076) under a nesting bound, applies the promotions of
   pages 1061–1062, and evaluates every expression of the grammar: the operators with their null
   and error rules, `var` in its blocks, user-defined functions, `if`, the three loops, `break`
   and `continue`. It runs under a `Budget` of steps, call depth and string bytes, and page 380's
   advice that a processor have some means of ending a seemingly infinite loop is the step budget.
   It is to run in the confined script worker beside Boa, under the same budgets and the reader's
   `Scripts` level, `off` by default (ADRs 1904, 1616); no window supplies it yet.
   **A chain is bounded by its depth.** The parser counts nesting against `MAX_NESTING` (64) and
   keeps the depth of binary operators in the tree it builds, on any path from the root and
   through parentheses, blocks and arguments alike, within `MAX_OPERATOR_DEPTH` (200), because a
   chain `1 + 1 + …` is a tree one level deeper per operator, and a chain whose first operand is a
   parenthesised chain is as deep as the two summed, which the evaluator and the drop walk on the
   stack; a deeper script is the `Parse` refusal `OperatorsTooDeep`. The longest chain in the 502
   distinct FormCalc scripts the census printed whole is 7 operators, so the bound is about thirty
   times what forms write. `a_long_chain_of_operators_is_refused_and_never_overflows_the_stack`
   runs 100 000 terms flat, and
   `nested_chains_are_refused_by_the_depth_of_their_operators_and_never_overflow_the_stack` thirty
   parenthesised levels of 200 `+` and twenty levels of six chains of 199, each on a 256 KiB
   thread, where an unbounded walk aborts the process; twenty-five levels of eight, 200 deep,
   still run there.
3. **What is refused, by name and page, and never approximated**: every built-in but `Sum`,
   `Concat`, `Replace` and `HasValue` (the other 63 documented ones and the 13 undocumented ones
   page 1076's note names), a method call, `.*`, an accessor through a variable that holds a
   reference, the `..` and `.#` separators, a qualified reference other than `$`, and the `parent`
   property. A refused script assigns nothing it had not already assigned before the refusal, and
   the host sees every earlier assignment.
4. **A string comparison is by code point, a departure.** Page 1063's comparison of two strings
   is locale-sensitive lexicographic, and this crate holds no collation. Equality is the same
   under both and no calculation of the census compares two strings; an ordering of text a locale
   collates differently from its code points is where the two differ. The departure is stated here
   and in `eval.rs`; the locale the `pdf-model` side already reads (ADR 2414) is what a later
   round would pass in.
5. **The hook is in `pdf-model` and `pdf-model` does not depend on `pdf-formcalc`.**
   `Resource::form` returns a `Form`: the template's `calculate` scripts in document order with
   their language, a `resolve` that follows page 114's search (children, then the container and its
   siblings, then the parent and its siblings, up to the root) and page 115's inferred index, the
   values the fields carry typed by their `value` element, `set`, and `edits`, which names each
   assigned field by its PDF name. The `Host` that joins a `Form` to the evaluator is written by
   the program that runs scripts, so the layering of `doc/crate-map.md` is kept: the new crate is a
   leaf, and neither side knows the other. `tests/common/hosted.rs` is that adapter for the tests
   and the census, and the script worker is where it will live.
6. **Runs at the server do run.** A `runAt="server"` script is run: page 411's note says a
   processor that works on a local file takes both roles.
7. **A defect found by the hook is fixed here.** `Resource::read` of a whole-resource stream took
   the first `template` element at any depth, and a `config` packet before the form's has one of
   its own (page 1335), so a form with such a packet had no containers at all, its fields
   unresolved and its saves unwritten. The form's template is the `xdp:xdp` root's child.
   `a_configs_template_element_is_not_the_forms_template` fails without the change.

**Designed, not built this round**: the activation order of page 379 (events, then calculations,
then validations, with the unordered scripts of one event in page 404's order), the cascade by
dependency of pages 379–380 with the step budget as its end, the `override` conditions of a
calculation, and the result: a field's calculated value becomes an edit in the script outcome
beside a person's typing, and the edit reaches the `datasets` packet through the save of ADR 2396.
JavaScript calculations and every event are step 5's.

## Evidence

The census's 24 FormCalc calculations are run once, in document order, against their own forms
through the hook and the adapter: all 24 evaluate, none refused, none raising an error and none stopped by a budget — 14 change the value their field held, 10 leave it as it was.

`crates/pdf-formcalc/tests/chapter_25.rs` runs the chapter's examples with the page each value is
derived from (the expression list of page 1060, the promotions of page 1061, the function and the
for loop of pages 1066 and 1073, the declarations of page 1075, `Sum`, `HasValue`, `Concat` and
`Replace` of pages 1090, 1117, 1122, 1132 and 1135) and the budgets; `tests/hybrid_form.rs` runs
scripts through a real `Resource`; `crates/pdf-model/tests/xfa_datasets.rs` tests the hook. The
fuzz target `formcalc` holds the budgets, determinism and the refusal's silence. A ten-minute run from 102 seeds reached 1 954 coverage points and 8 732 features in 17 124 178 executions without a failure.

## Consequences

Habit proposed: **a census that an RFC took over a small corpus is re-run over the crawl cache
before its finding of none decides an order.** RFC 0011 section 4's finding of no calculation was true of 1 479
files and false of 90 319, and it had fixed step 5 before step 6 on it.

§K.2 stays `partial`: the 24 calculations evaluate, but no window runs them yet, and the
JavaScript calculations and the events are owed.
