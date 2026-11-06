# 2728 — The Ghent PDF Output Suite is a pinned fetch, judged in both views, and its decidable patches are a ratchet

Status: accepted. Session 1957. Context: the owner's words of 2026-11-06 ("If freely available, please add Ghent
Workgroup (GWG) conformance test suite to our tests", and "we should treat gwg similar to the other test-corpus
repositories"); ADR 2656 (`tools/places.sh`); CLAUDE.md principle 5; ISO 32000-2 §8.6.7, §10.8.3.

## The licence decides where the suite lives

The suite's legal notice (© 2016 Ghent Workgroup) permits use for "its intended purpose of testing workflow setup",
forbids sale and commercial use without the Workgroup's written permission, and requires the notice in every copy of
the whole or a substantial part. Drawing the patches to see whether this program processes them is that purpose. A
copy committed to this public, Apache-licensed repository is not: it would carry the suite under terms that allow
what the notice forbids. So:

- no byte of the suite is tracked — no PDF, no README, no rendering of a patch;
- the other corpora are submodules under `doc/corpora/`; this one has no repository, so `tools/fetch-gwg.sh` is the
  submodule's equivalent: two URLs and two SHA-256 pins, a refusal on a differing hash, an unpacking into the
  git-ignored `doc/corpora/gwg-output-suite/` with `NOTICE.txt` beside the suite's own legal notice;
- `tools/places.sh` names it `PDF_VIEWER_GWG`, defaulting to that directory and overridable like every place;
- `doc/gwg-output-suite-provenance.md` records the URLs, the date, the hashes and this reading.

## Two views, and which one a patch is judged in

The suite aims at PDF/X output: overprint, spot colourants, an output intent. This reader has two views of a page,
the default screen view and §10.8.3's simulation of separations, which a person asks for. Every patch is drawn in
both. §8.6.7 says overprint has effect only where "the output device supports overprinting" and that otherwise "the
value of the overprint parameter shall be ignored" (its NOTE 1), so a patch whose `X` exists to reveal ignored
overprint is *not applicable* in the screen view, which honours the clause by showing it, and is judged in the press
view. A patch that does not turn on overprint is judged in both. Where GWG's expected result and a clause disagree,
the clause wins and the table says so.

## The harness is a ratchet, not a target list

`crates/pdf-model/tests/gwg_output_suite.rs` draws the patches a pixel comparison decides — two windows the README
says must agree, or a photograph that must be continuous-tone — and asserts the outcome *recorded today*, failures
included. A fix flips a check and fails the test until `doc/gwg-output-suite.md` and the expectation move together;
a regression fails it the same way. It skips with a sentence where the suite is absent (CI). `tools/gwg-render.sh`
draws every page in both views for a person, refuses an output directory inside the tree, and builds the sandbox
worker, without which JBIG2 and JPX draw nothing (trap 10).

## Consequences

- The table is this program's reading in its own words; nothing of GWG's text is quoted beyond a patch's name.
- A patch whose verdict is "not applicable" in one view is not a deficit in it; the press view carries it.
- The coordinates in the harness are read off renders that the tree does not hold; a new suite release re-reads them.
