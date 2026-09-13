# Q62 — Withdrawn: the two rewrites are reached by documents on this disk

Source: the merge of rounds 999–1004 (session 1005). **Withdrawn by session 1007, which measured
the thing this question asserted.** The file is kept rather than deleted because the retraction is
the useful part: this question asked the owner to decide the fate of two rewrites on the strength
of a claim nobody had counted.

## What this question used to claim

That `Rewrite::PostScriptXObject` (ISO 19005-2 section 6.2.9.3) and
`Rewrite::SymbolicTrueTypeEncodingRemoved` (section 6.2.11.6) were, after session 1001 made
section 6.2.2's exemption readable, **reachable by no document** — and that the owner therefore had
to choose between leaving two unexercised rewrites in the tree, retiring them, or reopening
ADR 0816's fence so the drawn case could be tested.

## What is actually true

Session 1007 built `crates/pdf-archive/examples/withdrawn.rs` and ran it over seven corpora
(veraPDF, `doc/pdf.js`, the four `doc/corpora` submodules, `doc/corpora-own`). Both premises fail,
and both fail **on the drawn side** — the side this question said was empty:

| rewrite | documents failing the row | withdrawn by the exemption | standing | the converter's answer is the rewrite | wrote a file with it applied |
|---|---|---|---|---|---|
| `PostScriptXObject` | 5 | 1 | 4 | 5 of 5 | 2 (`isartor-6-2-7-t01-fail-a.pdf`, `6-2-9-3-t01-fail-a.pdf`) |
| `SymbolicTrueTypeEncodingRemoved` | 43 | 1 | 42 | 18 | 7 |

So there is no decision to take: **nine documents on this disk are converted with one of these
rewrites applied**, and retiring either would remove a remedy that runs.

The third option was misdiagnosed too. Of the 43 symbolic cases, **25 are refused by
`sites::selects_the_same_glyphs`, not by ADR 0816's fence** — so "reopen the fence" would not have
reached them, and the fixture limitation this question blamed was not the binding constraint.

## What went wrong, and it is the reusable part

The question generalised from the **test fixtures** to the world. Both rewrites had been built
against fixtures declaring a resource their page never drew; when session 1005 rewrote those tests
to assert the exemption, the tree contained no *test* reaching the rewrite, and that was read as no
*document* reaching it. The corpora were never asked. `withdrawn.rs` exists now because the
question could not be settled without an instrument that counts reach per requirement, and it
found, in passing, that **eleven of 175 implemented rows fail no document at all** — the other way
a rule goes unjudged, and one no exemption is responsible for.

Trap 25's shape, in a new place: a population asserted from what was in front of the author.
