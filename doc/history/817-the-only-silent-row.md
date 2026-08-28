# 817 — The only `silent` row, and a list that carries an earlier one on

Date: 2026-08-28. Branch `round-817`, from `main` at `22e1feef`. Parallel round, worktree `r817`.
ADR: [0748](../adr/0748-a-list-that-carries-an-earlier-one-on.md).
Touched: `crates/pdf-model/src/structure.rs`, `crates/pdf-model/examples/list_continuation_census.rs`
(new), `crates/viewer-core/src/accessibility.rs`, `crates/viewer-core/tests/accessibility_census.rs`,
`crates/viewer-confined/src/protocol.rs`, `crates/viewer-confined/src/protocol/panels.rs`,
`crates/viewer-accessibility/src/tree.rs`, `crates/viewer-accessibility/tests/tree.rs`,
`crates/viewer-core/tests/headless.rs`,
`doc/conformance/ledger.toml` (§14.8.5.5), `doc/ledger-and-claims.md`, `doc/errata-read.md`
(one row's closing sentence), `doc/state-of-play.md`, and two new files, `doc/adr/0748` and this one.

## The subject, and why it was taken

The batch's general-improvement round, told to let the instruments name the subject and to keep off
three siblings' lanes (`doc/todo/56`, the errata ranking, fuzzing). The instruments were asked over
the pristine tree, and one of them names a defect outright rather than ranking one:

**`cargo run --release -p conformance --bin ledger` printed a `silent` row, and exactly one.**
`doc/ledger-and-claims.md` defines the status as *not implemented, and nothing says so*, which is
the one status this tree treats as a defect rather than as a position — principle 3's "unsupported
input must stay loud", `doc/traps/parsers-and-streams.md`'s trap 5 and `doc/todo/README.md`'s
closing sentence are the same rule three times. The row was §14.8.5.5, put there six rounds earlier
by ADR 0743 when an erratum put a reader in front of Table 382's two PDF 2.0 entries for the first
time.

Nothing else the instruments printed was a defect. The sweeps' hits were the shapes their own
catalogues call noise; `doc/todo/README.md`'s band 10–19 holds items whose remainders are priced
and parked; the corpus's `incomplete` list is classified exhaustively. A status whose whole meaning
is *this is owed and nothing says so* outranks all of it.

## What the clause turned out to say

§14.8.5.5 splits its two paragraphs by audience, which is what settles the row. The first is about
`/ListNumbering` and the `Lbl` elements — a scheme for a processor that intends to renumber, and
the argument for calling it inapplicable stands. The second is addressed to whoever *interprets an
`L` element*, which is this program the moment `StandardType::List` reaches a screen reader as a
list. `/ContinuedList` is a flag saying the list carries an earlier one on; `/ContinuedFrom` names
which; and where the flag is set with no identifier the clause names the predecessor itself, "the
preceding list at the same level in the structure hierarchy".

So a listener was being told a fresh list where the file said *continuation* — and the numbering
starting again at 1 is exactly what a producer writes the entry to contradict.

## What was built

`Tree::list_continuation` reads both entries and `list_predecessors` resolves them over a walk,
applying the clause's fallback where no identifier is stated. `AccessibilityNode` gains
`continues_a_list` and `continued_from`, both of which cross the confined pipe;
`viewer-accessibility` says the first in the node's description and publishes the second as
AccessKit's `FlowTo`, from the predecessor to the list that carries it on. ADR 0748 has the
argument, including the one place the standard leaves a choice — a list stating `/ContinuedFrom`
and no flag — and why an explicit `/ContinuedList false` beats it.

Errata Collection 3's Issue #346 is load-bearing rather than decorative here: its carets put *not
inheritable* into both cells and into neither of `/ListNumbering`'s, which is why the reader asks
`Tree::attribute` and not `Tree::inherited_attribute`. A test element nested inside a continuing
list, with its `/P` stated so that inheriting would have somewhere to go, is what pins it.

## What the corpus said, and it said nothing

`pdf-model --example list_continuation_census` is new and walks a structure tree counting both
entries, both resolution routes, and the probe trap 11 asks for — elements stating either entry
that are not `L` elements. Over the pdf.js corpus, `doc/` and the four corpora: 1245 documents
opened, 153 tagged, 1566 `L` elements, none stating either. Over the SafeDocs crawl: 65 967 opened,
23 501 tagged, 196 297 `L` elements, none stating either. Not one misplaced attribute anywhere.

That is §14.8.5.6's position exactly, and it is why this belongs to the coverage question. The
zero is recorded rather than glossed, and the instrument is committed so the next round can ask
again.

## The gates, the sweeps, and one thing worth knowing

The full §2 sequence ran, and three lines of it had to be run twice. The formatting gate and
`clippy` each found something the change had introduced, and one is worth writing down: **a
verbatim quotation of a spec entry trips `clippy::doc_markdown`.** `ContinuedList` and
`ContinuedFrom` are CamelCase, so clippy asks for backticks inside the quotation marks — which
would make `every_quotation_is_the_standards_own_words` fail, because the standard does not write
them that way. `#[expect(clippy::doc_markdown, reason = "a verbatim quotation …")]` is the tree's
own precedent, on `DEFAULT_STANDARD_NAMESPACE` and its bare URL, and it is trap 7's `#[expect]`
rather than `#[allow]`. The conformance gate then caught a blockquote with no clause cited before
it, which is its own rule working.

The `§4` sweeps ran on both sides, the baseline built in a checkout of `22e1feef` with a build
directory of its own — the second method `doc/todo/01` recommends. `blockers` and `parts` were
byte-identical; every other delta is line-number shift from two added ledger lines, or a count this
round is the cause of, and each was read: `callers` 334 → 336 `pub fn` with *both* named by a
dependent crate; `tables` 2452 → 2467 agreeing key citations with **absent and contradicted
unchanged**, which is the ninth sweep checking this round's own Table 382 citations; `quotations`
2847 → 2856 verbatim with **diverging unchanged at 38** and the ledger's at 2; `pointers` +9 with
`absent` and `undefined` unchanged; `entries` 292 → 293 rows explaining themselves by an arrival and
naming code, which is the row itself.

Trap 13's calibration was run on all four new tests, eight plants in all, each reverted: the entries
made inheritable, the level ignored in the fallback, an explicit `/ContinuedList false` no longer
winning, the `FlowTo` relation published on the wrong end, the two description sentences collapsed
into one, the predecessor dropped on the wire, the prune's remap removed, and the resolution never
written into the answer. Every one failed the test it was aimed at.

**The fourth test is the one the round nearly did without.** `pdf-model` reads the entries and
`viewer-accessibility` publishes them, and each had a test; what neither covered is the step
between, where `viewer-core` resolves the predecessor over the *whole* walk and then prunes
everything not on this page — so the index the resolution produced is not the index the answer
uses. The fixture puts an element belonging to page two in front of the two lists, and without the
remap the answer points a client at the paragraph between them. Nothing in the corpus could have
found it, because nothing in the corpus states the attribute at all.

The census's count entered `accessibility_census` as a **printed** figure and not a ratchet, on
`doc/todo/05`'s rule.
