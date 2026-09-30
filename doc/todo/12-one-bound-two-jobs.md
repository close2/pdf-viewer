# One bound doing two jobs: the differing fraction on text pages

Status: **answered, and nothing here is owed.** The two jobs stay one number: they are not two
jobs a threshold can separate in *either* direction. The argument is the ADRs below, every
paragraph this file held is in one of them, and the two re-run commands are `doc/verify.md`'s
(its *two re-runs of the bound the oracle judges a text page by*), so the file is its header
(ADRs 1416, 1439).
Cited by: §8.9.5's ledger note and `doc/traps/oracle-and-references.md`, which point at the
question by this file's number; every other citer points at the ADR that answers it. The
`doc/todo/12` in `doc/QUORRA_FEEDBACK.md`, `doc/todo/01` and `tools/conformance/src/pointers.rs` is
an earlier item of the same number, deleted long before this one was opened.
Priority: 12 — demand-driven, and it is about the instrument rather than about a page
Code: `tools/pdfref/src/lib.rs` (`Tolerance`, `Judgement`, `widened_to`),
`tools/pdfref/src/reference.rs` (`substituted_cmyk_profile`),
`crates/pdf-model/tests/oracle.rs` (`the_fixed_bounds_against_the_references_own_spread`,
`substitutions_of`, `print_the_substitutions`, `the_excluded_reference_under_the_same_bound`,
`name_the_pages_the_excluded_reference_survives`, `ConsensusIdentity`,
`the_consensus_that_decided_it`, `what_the_consensus_was_made_of`, `RaisedFormation`,
`a_raised_formation_bound`, `what_the_new_convictions_are_made_of`,
`the_pages_a_raised_formation_bound_would_move`), `tools/pdfref/src/lib.rs`
(`Triangulation::rejudged`, and `decide`'s two tolerances)
Derivation and numbers: **ADRs 0243, 0717, 0771, 0772, 0773, 0774 and 0776**. Read 0771 first; it
supersedes the reason the other two give for leaving the bound alone without changing what they
measured, 0772 corrects two populations it stated in prose, 0773 reads the vector row it handed on,
0774 answers the question 0773 handed on by widening its denominator — which is 0771's own
general shape arriving one round later — and **0776 does the same for the consensus half**: the
278 are composed rather than counted, and the formation bound turns out to move our own floor
through `widened_to`.
Two neighbouring questions: **ADR 0575** (a consensus of two is the same evidence as one of three,
one factor less) and **ADRs 0616 and 0617** (a verdict is one every maximal consensus reaches, and a
page whose sets divide about us is `ambiguous`).
