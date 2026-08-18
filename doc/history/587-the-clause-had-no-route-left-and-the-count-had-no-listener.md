# 587 — The clause had no route left, and the count had no listener

**Finding:** §9.10.2's three methods are all walked, per code, and the two whose order could
theoretically differ are conditioned on being a simple font and a composite font — so the 1226
unnamed codes are the clause's own "there is no way", and what was owed was never a fourth method
but a *listener*: the count now crosses to a host, to a screen reader and to `pdf-retrieve` without
becoming a report.

Date: 2026-08-18. ADR: [0422](../adr/0422-the-clause-had-no-route-left-and-the-count-had-no-listener.md).

Touched: `crates/pdf-model/src/content/report.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/examples/unnamed_code_census.rs`, `crates/viewer-core/src/query.rs`,
`crates/viewer-core/src/viewer.rs`, `crates/viewer-core/src/open.rs`,
`crates/viewer-core/tests/headless.rs`, `crates/viewer-confined/src/lib.rs`,
`crates/viewer-confined/src/protocol.rs`, `crates/viewer-accessibility/src/tree.rs`,
`crates/viewer-accessibility/tests/tree.rs`, `crates/viewer-ui/src/bin/pdf-viewer/access.rs`,
`tools/pdf-retrieve/src/lib.rs`, `tools/pdf-retrieve/src/main.rs`,
`tools/pdf-retrieve/tests/retrieval.rs`, `doc/conformance/ledger.toml`, `doc/HANDOVER.md`,
`doc/todo/21-font-substitution.md`.

## Both halves of the round were the same clause family

The demand-driven half was `doc/HANDOVER.md`'s open question — what to do with the unnamed-code
band — and the spec-driven half was §9.10.2's row and its neighbours. They are one piece of work
because the decision could not be taken without the reading: if a method of the clause were
unimplemented, the band would be a defect and the answer would be to implement it, and no amount of
arithmetic about the oracle's judged set would matter.

So the reading came first, and it is the part worth keeping. The census's split was re-run and read
with the trace rather than reasoned about; `spec-errata emit` was run over clause 9 before anything
was written, and the collection annotates fifteen subclauses of clause 9 and **none of §9.10.2**;
each of the three methods was checked against `LoadedFont::text` for being present, for being tried
in the clause's order, and for falling through *per code* rather than per font. The one place a
trap-5 defect could have hidden is the third method's condition, whose exception "(except Identity
-H and Identity -V )" governs its first disjunct alone — an `Identity-H` font whose descendant
states a registered collection is inside the method — and `collection_meaning` reads the
descendant's `/CIDSystemInfo` without ever consulting the encoding's name, so it is honoured.

## The thing worth remembering is the shape of the question

The handover's sentence weighed two options, report or silence, and both are wrong for the same
reason from opposite ends: reporting claims this program failed at something the standard says
cannot be done, and silence hides a shortfall from every consumer that has one. The third option
was in nobody's list, and `Interpretation::codes_without_a_character`'s own doc comment had been
promising it for eleven rounds — "a host that searches or selects can read it and say so" — while
no host could, because it never crossed `viewer-core`'s boundary. **A doc comment that describes a
capability a consumer has no way to reach is a claim, and it decays like any other.**

What the round built is four listeners for a number that was already correct: `Shortfall` on
`Interpretation`, `Query::Readback` beside `Query::Reports`, a sentence in §14.7's status group for
the one population the shortfall is completely invisible to, and a `readback` object in
`pdf-retrieve`'s JSON beside the text and never inside it. The two sentences a person can hear are
worded apart deliberately, and a test asserts the second does not say the picture is wrong.

## Numbers

Off the runs, not off a document. The corpus gate's third silence line and the census agree at
**1226 codes over 41 documents**, which is the check that they count one population. Both text
gates, the oracle's verdicts, quorra and the corpus's incomplete count are unchanged, which they
have to be: nothing this round added runs during interpretation. `tools/state.sh` prints all of it.

One latent flake was fixed on the way past — `pdf-retrieve`'s test fixture named its temporary file
after the process alone and two tests in one binary raced on it — which is the ordinary way a
shared-path race is found: by adding a test that changes the scheduling.
