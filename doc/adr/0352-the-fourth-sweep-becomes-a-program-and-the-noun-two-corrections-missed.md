# 0352 — The fourth sweep becomes a program, and the two ADRs a correction walked past

**Status.** Accepted.

## Context

`doc/todo/01`'s binding rule for a sweep round: commit one more prose sweep as a program before
running any of them. Five of the fifteen were commands — `conformance --bin entries`,
`--bin quotations`, `--bin unread`, `--bin blockers`, `--bin capabilities` — and ten were
descriptions. The retired-claim sweep is the highest-yield of the ten and has been since the
two-hundred-and-sixteenth session: §8.9.6.1 still saying a soft mask was "reported rather than
applied on 28 corpus documents" fourteen sessions after §11.6.4.3 retired that sentence;
§12.5.6.22's `/FixedPrint` explained by the refusal ADR 0168 dismantled; four rows and four
source comments saying this tree reads no `XMP` after the round that gave it one, three of the
six written *by* that round. Its subject is the commonest shape in that file — **two places
describe one mechanism, and correcting one leaves the other lying.**

## Decision

**`cargo run --release -p conformance --bin retired -- <noun> …`** is the fourth sweep as a
program (`tools/conformance/src/retired.rs`, nine unit tests). Three decisions are worth their
lines, because each is a place where this sweep is not like the other five.

**Its population comes from the caller, and no program can derive it.** Every other sweep here
finds its own subjects: a blocker phrase, a capability phrase, a `/Key` a note calls unread, a
table number, an entry the clause's own table states. This one asks *where else* a claim the last
rounds retired is still written, and what those rounds retired is a judgement they made. So the
nouns are arguments — one run per sweep round, exactly as the by-hand runs took their list from
the round's own record — and `doc/todo/01`'s standing refinement carries over unchanged: give it
the **mechanism** (`NoZoom`, `uncoloured`, `/AIS`), not the sentence, because the sentence that
stayed is rarely worded like the sentence that went.

**What a program settles that the grep could not is the *order* to read the hits in.** A grep
prints matching lines. The judgement a person made on each one has a shape: a round retires a
claim by recording the retirement where it makes it — "this row said X until the four-hundredth",
"since ADR 0234", "no longer" — and leaves the plain claim standing somewhere else. So each
mention is classified `Correction` or `Standing`, and **a noun carrying both shapes is the
defect's own signature**. A noun mentioned only in corrections is a round that swept its own work
properly, which is a result rather than a silence; a noun mentioned only in standing claims is
prose nobody has had cause to correct.

**It reads more prose than the by-hand runs did, and one directory less than it could.** The
targets are `ledger.toml`, every `//` comment under `SOURCE_ROOTS`, and every Markdown document
under `doc/` this project wrote **except `doc/history/`**. Wider than `doc/adr/`, which ADR 0265
added, on the evidence of the five-hundred-and-first: its second finding was
`doc/todo/README.md`'s index line for an item closed one wave earlier — *an index row decays at
its item's pace, not its own*. Narrower by `doc/history/`, and that is not an oversight: a history
file is one round's record and no round edits another's (`doc/todo/02` §6), so a retired claim
standing in one is not a defect to correct but what that round wrote on the day it wrote it.
Reading them would put every round's account of every noun in front of a reader with nothing to do
about any of it.

Not a gate, on ADR 0249's ratio argument and one of its own: its input is what somebody typed on
the command line, and a build failure may not depend on that.

## First run, and both defects are in ADRs

Seventeen nouns from sessions 511–516: `RadiosInUnison`, `an_earlier_button_answers_to`,
`ControlFit`, `render_retained`, `RetainedScene`, `is_bare_cff`, `embedded_program`,
`each_addressable_code`, `chord floor`, `REFUSED_AT_FOUR`, `CONTRADICTED_CALRGB_TO_SCREEN`,
`hollow`, `CIDToGIDMap`, `Arabic`, `joining`, `prefix`, `Corrupt`. **544 mentions, seven nouns
carrying both shapes, two defects — and both are in `doc/adr/`, the population ADR 0265 added and
the one with no gate at all.**

- **ADR 0235's consequence still read "`RadiosInUnison` crosses and is not obeyed", with the
  division called deliberate.** The five-hundred-and-eleventh session read that sentence out of
  §12.7.5.2.3 and found it wrong in both directions — the flag *set* was obeyed by construction,
  the flag *clear* was violated — and corrected two ledger rows and `doc/todo/30`. **Its own
  history file names ADR 0235 as the fourth place carrying the claim**, and the ADR was left
  standing. Four places, three corrected, six rounds.
- **ADR 0337's "what this does not do" still filed `freetext_no_appearance.pdf` under
  `doc/todo/21`'s per-character fallback.** ADR 0348 (session 513) read the witness out: no
  compiled-in face carries one Arabic glyph, so a per-character chain has nothing to chain to, and
  what that document needs is a glyph source, joining-form selection and right-to-left ordering
  together or not at all. `doc/todo/21`, `doc/todo/22` and §12.7.4.3's row carry the corrected
  filing; the ADR did not.

Both amended here, in the commit that found them, which is ADR 0265's rule.

**And the run taught the program one rule about this project's own notation.** `doc/todo/30`'s
closed items and `doc/todo/01`'s "what is still owed" list strike the retired sentence and write
the correction *after* it, so a sweep reading one sentence at a time files the struck half as a
standing claim — the opposite of what the marks say, and what the amended ADR 0235 produced on
the re-run. A sentence containing `~~` is a retirement, whatever words are inside it.

**One thing the run says about the instrument rather than the tree.** A noun that is also an
ordinary English word costs the run its signal: `prefix` returned 262 mentions and `joining` 36,
almost all of them threads being joined and lists being joined. That is the one-short-key noise
shape wearing a gerund, and the answer is the same as everywhere else here — read the sentence —
but it is worth knowing before choosing a noun. The nouns that paid were the ones a session
*invented*.

## The eighth sweep paid too, and on a pointer written a hundred sessions after the file went

`crates/viewer-ffi/src/form.rs` cited `doc/todo/37`'s audit as the thing this module answered.
The four-hundred-and-ninth session deleted that file when `Command::Delegate` closed its last
item; the four-hundred-and-thirteenth found the *live* claim behind the same dead pointer in
`viewer-gtk`. This citation was written in the five-hundred-and-eleventh, and it is a plain
citation rather than a correction quoting the pointer it retired — the shape the other three hits
have. The audit's argument is ADR 0235's, and the comment says so now.

## The blame band from commit 213, and the row that turned into work

Twelve rows read against the code, oldest first: §12.7.6.4, §14.8.2.1, §7.6.4.3.2, §12.3.2,
§8.11, §9.8.2, §12.8.2.3, §12.3.2.2, §12.3.4, §12.7.3, §12.5.6.5, §12.2. **Two were wrong, one of
those was a `shall` nothing in the tree had read, and one status moved.**

**§14.8.2.1 said "a selection is still taken in content order, and the map between the two
offsets is what remains".** `Tree::logical_range` *is* that map, `Query::LogicalSelection` answers
with it, and `viewer-ui` has copied by it since the four-hundred-and-thirteenth — and
§14.8.2.5's own row has said so in those words ever since. The seventh failure shape: two rows
about one mechanism, disagreeing, one family apart. Every rule this clause states is a pointer to
a subclause and all six are answered, so the row is **`implemented`**, with the logical-order
tests named as evidence.

**§12.3.2.2's `partial` was for `Target::Number` and the reading found a different debt and a
departure.** The integer page number *is* read, and `Destination::page_index_in_target` resolves
it against §12.6.4.4's embedded target where Table 203 numbers the first page 0; what no reader
can resolve is a page number in a file it does not open, which is §12.6.4.3's refusal and not this
clause's gap. What nothing had read is the parenthesis in the same paragraph as the bounding box
every `/FitB` uses:

> (If any side of the bounding box lies outside the page's crop box, the corresponding side of
> the crop box shall be used instead; see 14.11.2, "Page boundaries" for further discussion of
> the crop box.)

`pdf_model::interpret` puts the displayed box **at the origin** rather than clipping content to
it, so the display list carries every mark the stream made, including the ones off the page's
edge, and `content_bounds` unioned all of them. A `/FitB` on such a page magnified to fit ink this
viewer never draws. `viewer_core::open::content_box` cuts the box to the page first, with the
clause's parenthesis quoted over it and a unit test for the three cases (inside, hanging off every
side, entirely outside — the last falling back to the page box, as a page covering nothing does).

**Ten rows were kept, each recording the evidence that kept it**, which is what moves the blame
pointer without a stamp: the grep for `AllCap` and `SmallCap` (nothing in the tree names either,
and every `Script` is part of *PostScript*), the grep for `"CO"`, `crypt.rs`'s shared conversion
and its overlay of the password on the padding string, §8.11.4.4 still being the row that names
what is missing, `Query::Preferences` still answering Table 147 whole. **One kept row gained a
precision instead of a correction**: §12.7.6.4 said what is owed is the clause's "or any other
data format that it supports", and what is owed is *XFDF*, which the clause names outright — a
processor satisfies the open-ended tail by supporting what it supports.

**And §12.8.2.3 gained the one entry of Table 258 nothing quotes**, from the entries sweep: `/Msg`
is "[a] text string that may be used to specify any arbitrary information, such as the reason for
adding usage rights to the document", which is the producer's sentence to a *person* — and
`notes.rs` is already where this program tells a reader what rights a document asserts. It states
no requirement, so it is named on the row rather than built, which is what the fifth sweep got
right about `Query::Find`.

## Consequences

- Six of the fifteen sweeps are committed programs; nine remain descriptions, one per sweep round
  until the backlog is gone.
- The ledger moves one row to `implemented`; `silent` stays zero and every gate is green.
- **The `partial` rows below commit 202 of 673 are read to the end, and the band from 213 to 409
  now is too.** What remains under the fold is §12.6.4.4 at commit 184 — a row last written by the
  session that built the clause it describes — and the read-and-kept set, whose evidence is in
  their notes. The next band begins at §7.9, commit 413.
- The `/FitB` change is the only one that reaches what a person sees, and it changes a
  *magnification* rather than a mark: no corpus or oracle page is drawn differently, because
  neither gate applies a destination.
