# 0989 — The other document this tree quotes verbatim, and the sentence it retired

Session 978. Status: **accepted**. It reads the tree's quotations of `CLAUDE.md` against
`CLAUDE.md`, corrects the twenty-four places that quoted a sentence it retired ninety-two rounds
ago, builds the sweep that finds them, and removes a dependency `pdf-syntax` has declared and
never used since its first commit.

## 1. The question, asked one document over

`CLAUDE.md` states the rule twice and only one half had a reader:

> **Quotation marks mean verbatim.** A load-bearing normative sentence goes in as a rustdoc
> blockquote, exact, under its clause number, so that the conformance checker can verify it
> against `doc/md/`.

`tools/conformance`'s `--bin quotations` is that checker. Its discriminator is stated in its own
header: it prints every quotation that "matches one of the specifications for at least
`MIN_MATCH` words and at least half its length **and then diverges**". A quotation that matches
*no* specification produces nothing, because that is what an ordinary sentence of this project's
own prose looks like — and it is also what a quotation of a **different document** looks like.

This tree quotes two documents verbatim, not one. The second is the file that decides what is in
scope, and it moves: `CLAUDE.md`'s authoring exclusion was redrawn on 2026-09-03 by the owner's
ratification of RFC 0002 §11.1 (ADR 0816, session 886). The sentence it replaced was

> no clause whose requirements fall on a generator is in scope: linearisation, object-stream
> packing, optimisation

and `CLAUDE.md` has not contained it since. **Twenty-three conformance-ledger rows went on
quoting it as `CLAUDE.md`'s, and a module header in `pdf-syntax` paraphrased it**, for the
ninety-two sessions between that day and this one.

## 2. Why it matters more than a stale quotation usually does

Two of the sentence's three nouns had by then been crossed **by argument**, which is the only way
`CLAUDE.md` permits an exclusion to move:

| the retired sentence called out | where this tree stands |
|---|---|
| linearisation | still excluded — `CLAUDE.md` says so in as many words: "Annex F stays excluded until linearisation is separately ratified" |
| object-stream packing | **implemented**: `pdf_syntax::serialize` generates §7.5.7 object streams (session 900, ADRs 0817, 0842) |
| optimisation | **shipped**: `optimize` is one of RFC 0002's five writing verbs |

So `crates/pdf-syntax/src/write.rs` opened by telling a reader that `CLAUDE.md` "excludes
*authoring* a document: linearisation, object-stream packing, and everything else whose
requirements fall on a generator", and that it "permits exactly one kind of writing, and this is
it" — with `serialize.rs`, **in the same crate, in the same directory**, generating object
streams and writing whole files. Nothing in the tree disagreed with either sentence out loud.

The twenty-two Annex F child rows are the same defect at scale, because they are boilerplate: one
paragraph written once and repeated per subclause, so a single retired quotation became
twenty-two of them in one sitting. That is the block shape `--bin tables` was built for one
population over (`doc/todo/02` §4) — a run of rows written together against an older text — and
it is worth naming as a general property rather than as a coincidence: **a ledger note that is
boilerplate multiplies whatever was wrong with it by the size of the clause family.**

## 3. What the ledger and the code now say

The twenty-three rows quote `CLAUDE.md`'s current sentence instead, which is exact and is the
*narrower* ground the aggregate row had already found for itself in session 886:

> Annex F stays excluded until linearisation is separately ratified

Annex F's status does not move. It was `out-of-scope` with `exclusion = "writer-side"` before and
it is that now; what changed is that the reason it states is a sentence the governing document
contains. The aggregate row records the correction, and §7.5.6's row records what `write.rs` said,
because `write.rs` is in that row's `code` list.

`write.rs`'s header now names both writers, cites the current exclusion — "[n]o clause whose
subject is deciding what marks a page should contain falls on this project" — and keeps the wrong
sentence in view under it, for the reason this project keeps every correction in view: a header
that simply becomes right teaches nothing about how it was wrong.

## 4. The sweep

`tools/governing-quotations.py`, wired as `tools/state.sh governing` and catalogued in
`doc/verify.md`. It reads every tracked text file for a quoted span of five words or more
*attributed* to `CLAUDE.md` — the mention running into the opening delimiter within sixty
characters — and asks whether the span is in `CLAUDE.md`, or in any specification under `doc/md/`
(which is what a paragraph quoting the standard beside a mention of `CLAUDE.md` looks like, and
is `--bin quotations`' business rather than this one's).

Three decisions are worth recording, because each was made against evidence rather than taste.

**Quotation marks are the population, and nothing else is.** `CLAUDE.md` makes them mean
something; a paraphrase without them is prose and is nobody's finding. This is what keeps the
sweep from being an opinion about wording.

**Attribution is proximity, and proximity is why it reports rather than fails.** The first draft
took every quoted span in a *paragraph* naming `CLAUDE.md`: 1 882 quotations read, **1 010
findings** — trap 39 exactly, a signal that always fires and looks like caution. Requiring the
mention to run into the quote took it to 156 read and 33 found. The residue that remains is
correct prose of three shapes — a document saying what `CLAUDE.md` *used to* state, a ledger note
quoting its own earlier wording, a mention that happens to sit sixty characters in front of
somebody else's sentence — so a finding is a question for a person, which is the verdict
`--bin quotations` reaches one clause over for the same reason. `doc/adr/`, `doc/history/` and
`doc/rfc/` are printed apart: a record quoting the sentence it retired is quoting it correctly,
and RFC 0002 is the document that *proposed* the change, so its copy of the old sentence is the
proposal.

**And it was calibrated against the defect, which is trap 13, and the calibration caught a bug
that hid the whole finding.** The sweep's second run reported fourteen findings and **not one of
the twenty-three ledger rows** — because a TOML note is `note = "…"` and the regular expression
paired that outer opening delimiter with the note's first *escaped* quote, so every real
quotation in the file was scanned as the space between two quotations. A sweep whose output had
looked entirely plausible was blind to the population it had been written for. The fix is four
lines and a comment; the lesson is the trap's own, and it is the second time in this tree that a
sweep passed by not looking (trap 25).

## 5. What the sweep still prints, and who owns it

Four residual findings are misquotations that this round could not touch, because four sibling
sessions hold their files:

| where | quotes `CLAUDE.md` as | what `CLAUDE.md` says |
|---|---|---|
| `crates/pdf-font/examples/type1_encoding_census.rs` | "a count that does not move is not evidence that nothing happened" | nothing — the sentence is `doc/todo/02` §7's |
| `crates/pdf-model/src/restriction.rs` | "off, on, ask before operations, warn before operation" | "`off`, `on`, *ask before the operation*, and *warn before the operation*" |
| `doc/todo/01-ledger-partial-rows.md` (twice) | "write down the command, not the answer" | "A fact that can be counted is not written down. What is written down is the command that counts it." |
| `tools/conformance/src/unread.rs` | the same phrase | the same |

None changes what any of them *does*; each is a claim about the project's rules that the rules do
not support. They are named here so that the round which owns the file can take them in a line.

## 6. The dependency that was never used

`crates/pdf-syntax/Cargo.toml` declared `pdf-spec` in the crate's **first commit** (`f26bbcb1`)
and no file in the crate has ever named `pdf_spec` — `git log -S` over `crates/pdf-syntax/src/`
returns nothing at all, so this is not a use that was removed. Removed.

It is not a performance claim and is not offered as one: in a full workspace build `pdf-spec` is
built anyway, for `pdf-model`. What it is, is the crate graph telling the truth — `CLAUDE.md`
principle 4 makes each crate's responsibility a stated thing, and a declared dependency is part of
that statement. The one measurable consequence is `cargo tree`'s: **`pdf-font` no longer has
`pdf-spec` in its dependency closure at all** (1 → 0), and `corpus-classes`, `spec-errata` and
`safedocs` reach it by one route instead of two.

## 7. What this does not claim

The sweep does not check paraphrase, does not read `raster/` (quorra's tree has a governing
document of its own that this program does not hold), and cannot tell a quotation *of* `CLAUDE.md`
from a quotation *near* a mention of it better than sixty characters allow. It is an instrument
for one failure — a sentence attributed to a document that does not contain it — and it found
twenty-four instances of that failure on its first honest run.
