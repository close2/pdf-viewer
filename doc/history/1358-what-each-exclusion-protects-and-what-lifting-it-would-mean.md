# 1358 — What each exclusion protects, and what lifting it would mean: RFC 0009

Date: 2026-10-05. Branch: `batch-1352-1357`, shared with rounds 1353–1355. No ADR: an RFC is the
deliverable and the owner decides. Commissioned by the owner: *"please use one of the next rounds
for a RFC for features we have excluded until now and what it would mean, to implement them now"*.

## What was written

**RFC 0009** (`doc/rfc/0009-what-each-exclusion-protects-and-what-lifting-it-would-mean.md`):
printing first, because the owner named it and it is not excluded — what RFC 0004 built, what it
left open, §10.6 re-read against a print path (the condition holds under every route proposed, and
the one route that flips it is named), §10.5, Table 147, §14.11.5 on paper, the two windows that
show rather than print, Route A re-priced now that the serializer exists; clause 13 in four tiers
with the libraries judged against `deny.toml` (the one pure-Rust audio stack is MPL-2.0; no U3D, PRC
or XFA crate exists); XFA on two grounds, with pdf.js's 16 379-line XFA layer read as a layout
engine; the authoring line in three steps with the cost of each; one pointer to RFC 0008; every
`inapplicable` and `writer-side` row read for a hidden fifth exclusion (none); a ranking; eight
current restrictions with their rationale; the costs; nine questions; what is built when; sources.

**`doc/questions/Q254`** points at the nine questions and recommends accepting all nine.

## Three things worth keeping

**Printing's remaining dependency lost its objection**: `ipp` 7.0.0 has a blocking `client` feature
over the `ureq` already in the lock, so the winit window's job submission needs no async runtime.
**The still half of clause 13 needs no engine**: §13.2.2's viability algorithm can be computed and
reported by a processor that plays nothing, and the media file's hand-off is ADR 1155's act under a
level. **XFA is a layout engine with a form vocabulary** — pdf.js's `layout.js` flows subforms across
pages — so it sits on the authoring exclusion as well as §K.1's permission, and `/NeedsRendering`
would put that engine on the launch path.

## Files touched

`doc/rfc/0009-what-each-exclusion-protects-and-what-lifting-it-would-mean.md` (new), `doc/rfc/README.md`
(one row), `doc/questions/Q254-does-the-owner-accept-rfc-0009-and-how-does-it-answer-its-nine-questions.md` (new), this file. No code, no ledger row, no `doc/todo/65`.

**Gates.** `cargo test -p conformance --no-fail-fast`: exit 0, every binary passing, including
`questions` and the citation gate over `Q254`. `cargo run -p conformance --bin quotations`: exit 0,
nothing named under `0009` or `Q254`. `--bin pointers`: exit 0, nothing named. `tools/batch.sh
check`: exit 0, every line `none` or `clean`. Load average 1.7 throughout.
