# ADR 0265 — An ADR is a dated record, and two of its sentences were not

Date: 2026-08-11 (session 429)
Status: accepted

## Context

Session 428 established, with `nm` rather than with an argument, that `confined_wire` does not
contain `pdf_model::interpret` and that `cargo-fuzz` has been installed on this machine since
26 July. Both facts contradict sentences in **ADR 0261**, written three rounds earlier. The
handover recorded that 428 "corrected ADR 0261's claim"; what it actually did was state the
correction in its own commit message and in ADR 0264. ADR 0261 still said it.

That is not an accident of one round. `doc/todo/01` carries twelve sweeps; every one of them
reads `doc/conformance/ledger.toml`, `crates/`, `tools/` or `fuzz/`. **Nothing has ever read the
264 ADRs.** `doc/todo/01` names the population — "every quotation of the standard in `doc/*.md`,
in `doc/todo/`, in `doc/HANDOVER.md` and in the 255 ADRs. Nothing reads any of it" — and files it
under "not owed until somebody has a reason to think it is wrong". ADR 0261 is that reason.

So the question this round had to answer is not "is there stale prose in the ADRs" — there is,
264 documents' worth — but **which of it is a defect**, and whether the answer is an instrument.

## Decision

**An ADR is a dated record of a decision, and its prose is not maintained. What binds is
narrower and it is a rule about rounds rather than a sweep: a round that disproves a claim
amends the ADR that made it, in the same commit.**

Three things follow, and the third is what makes the first two cheap.

- **The amendment goes at the top, in the `Status:` line**, which is the convention this tree
  already has: ADR 0135 ("Amended by ADR 0230 (session 393): the section … is closed"), ADR 0139
  (same shape, naming ADR 0219), ADR 0158 and ADR 0011 ("Corrected in session 202", "Corrected in
  the ninth session"). Four ADRs had found the form; nothing had written it down. A reader who
  opens ADR 0261 now meets the correction before the argument.
- **The body is not rewritten.** ADR 0261's paragraph stays as it was written, because the value
  of a decision record is that it says what was believed when the decision was taken. What
  changes is that the reader is told, at the top, which two of its sentences did not survive.
- **The instrument is the fourth sweep, not a thirteenth.** `doc/todo/01`'s fourth sweep already
  says to grep the *noun* a correction retired over every other row; the amendment to it is one
  word — grep it over `doc/adr/` as well as over `ledger.toml` and the source. That is the run
  that would have found this one: session 428's nouns were `confined_wire`, `interpret`,
  `cargo-fuzz`.

## Why not a sweep over `doc/adr/`

It was built and run before this was decided, which is the only honest way to decline it. The
same greps `doc/todo/01`'s first, third and eighth sweeps use, over all 264 documents:
**21 dead citations, 23 capability reasons, 20 expired blockers — 64 hits, and two are defects.**
Both are in ADR 0261 and both were already known from session 428's work.

The other 62 are correct, and correct in a way no discriminator can separate from the two:

- ADR 0107 *quotes* the ledger note it was retiring — "is the closest available approximation
  **while §11.4.6 does not exist**" — which is the same false positive the ledger sweeps have had
  since the two-hundred-and-sixteenth session, and here it is the ADR's whole subject.
- ADR 0162's "a window with scrolling and zoom, which this program does not have" is a blockquote
  of the row it corrected. ADR 0205's "a screen is not a printer" is the table of five wrong
  reasons it found. ADR 0199's "pane this program has no panel for" is the sentence it retired.
  **The stronger an ADR is, the more of the retired wording it contains**, so the noise grows with
  the value.
- The 21 dead citations are `doc/todo/NN-slug.md` paths whose files were deleted by the sessions
  that closed them — ADR 0169 naming `doc/todo/20-stencil-with-a-tiling-pattern.md`, ADR 0171
  naming `doc/todo/12-a-radial-shading-is-not-a-conical-gradient.md`, six naming `doc/todo/37`.
  Every one of them is a true statement about what the ADR was responding to at its date. A todo
  file that a round closes *should* be deleted, and the ADR that closed it *should* name it.

A sweep whose output is 64 hits and 2 defects, both of which the round before had already found
by other means, is a 32:1 ratio against a population that does not change. `doc/todo/01` prices
its ninth sweep's 5:1 as too noisy to gate and keeps it because it is *cheap and it pays*; this
one is cheap and does not. The rule above costs nothing and catches the same two.

## What the sweep does say, and it is worth one line

**264 ADRs, 64 hits, 2 defects.** A clean-enough run is a result the same way a clean sweep is:
it says the ADRs are not a hiding place for live false claims, and it says why — an ADR is
written once, by the session that made the decision, and never edited, so its claims are
timestamped by construction where a ledger row's are not. The failure mode `doc/todo/01` exists
for needs a document that is *maintained badly*. An unmaintained document has a different one,
and it is the one this ADR answers: a claim that was true at its date and has since been
disproved, standing with nothing on it to say so.

## Consequences

- `doc/todo/01`'s fourth sweep gains `doc/adr/` as a target. One word, no new instrument.
- ADR 0261 carries its amendment.
- The population `doc/todo/01` names as unread is now four items rather than five:
  `doc/*.md`, `doc/todo/`, `doc/HANDOVER.md` — and `doc/adr/` is read, once, with the result
  above written down so that nobody sweeps it again for the same reason.
