# 1807 — The range of field inheritance is unlimited, each field resolved once per document

Session 1807, Slot 2 of batch one hundred and twenty-one, a clause round; 2026-10-27. ADR 2448;
ADR 2449 and Q673 unused. I worked in `scratchpad/r1807/`. §12.7.4.1 `departed` → `implemented`.
**Read first.** ADR 2439 left §12.7.4.1's departure on its time half: a deep acyclic chain walked
once per widget. The premise held in the code, and the brief's "six walks" were fifteen: `Field::read`
and seven others up `/Parent` in `view/fields.rs`, `view/writing.rs`, `view/field_tree.rs`,
`submission.rs`, two `/AA` walks, and two recursive walks down `/Kids`, all at 256.
**Built (ADR 2448).** `pdf-model/src/inheritance.rs`: `Ancestors` walks a `/Parent` chain with
Brent's method, finding a cycle with two positions and no set, and has no bound; `Lineages` is kept
beside the document through a new `pdf_syntax::Document::derived::<T>()` (one memo per type, which
the syntax layer holds without naming) and resolves each field's thirteen entries — Tables 226,
228, 229 and 230's eight inheritable ones and the five the readers already took up the chain — and
its `/AA` by trigger, once per document. A field that states none shares its parent's by `Arc`.
Every walk up goes through it; the `/Kids` walks take an explicit stack, keep their sets and lose
the bound; `MAX_FIELD_ANCESTRY` and `view::MAX_FIELD_DEPTH` are gone. A cycle refuses the
appearance with "returns to a field it has already passed", and `Submission::owed` names it.
**One reading changed, with its clause.** Each entry is the nearest *stated* value (§7.3.9: null is
omission); readers had walked with their own type tests, `/Opt` and `/MaxLen` two each. A wrongly
typed value on an ancestor is now that field's statement; only a damaged file can tell.
**Hypothesis.** One resolver, held. Nothing moves on the arms: held, 0 differing pages of 968, 968,
968, 965, 965 and 965. The walk no slower: the first cut failed it. Over the corpus's 103 form
documents and 782 fields, HEAD measured 0.008/0.008/0.007 s and the first cut 0.010/0.010/0.009 s,
because a real chain is one to four links and copying every resolution cost more than the walk did.
With the `Arc` sharing it measures 0.009/0.008/0.008 s against HEAD's 0.008/0.008/0.009 s, in one
`--clock --nice 0` hold, load 2.3, no render-node holders.
**Proof.** Unit tests: cycles of one, two, and a tail into three; a 4 000-link chain with a widget
at every link; null inherited past. `tests/variable_text.rs`: a value 4 096 links up is drawn, and a
three-field loop is refused by name. Each is the other's control. A planted 512-link bound fails both
deep tests. **Left:** no test counts the memo's reads; `pdf-transform`'s and `pdf-signature`'s
walks keep their bounds (no interactive inheritance); the save's three update walks are unmemoised.
**Gates.** In an export of HEAD and my hunks (a sibling's `media.rs` broke the shared build):
clippy `-D warnings --all-targets` on pdf-syntax, pdf-model and pdf-transform exit 0; nextest on
pdf-model and pdf-syntax 2574 passed, 0 failed (exit 0, twice); rustdoc `-D warnings
--document-private-items` on both exit 0; `raster_golden` exit 0; `save_round_trip` exit 0 (ratchet
held, twice); the six arms exit 0 against HEAD's six arms. `rustfmt --check` on my files and `cargo
fmt --check` on the three crates exit 0. `cited` exit 0, `quotations` exit 0. `cargo test -p
conformance --no-fail-fast`: one red, 1808's 45-line record, is a sibling's. `names` is green after
I moved two `impl` bounds into `where` clauses. 7 600 s.
