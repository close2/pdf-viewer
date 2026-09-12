# 994 — The third amendment to the authoring exclusion, and RFC 0007 accepted on the answers

Date: 2026-09-12. ADR: 1014. Files: `CLAUDE.md` (the authoring exclusion under "What *done*
means": one paragraph added, one sentence of the amendment-history paragraph edited, nothing else),
`doc/adr/1014-a-page-of-the-documents-own-content-is-on-the-near-side-of-the-line.md`,
`doc/rfc/0007-a-refusal-is-a-question-somebody-can-answer-in-advance.md` (status, §2.1, §4.1,
§4.4, §4.5, §4.6.2, §4.7.2, §4.7.4, §5b.5, §6, §7 rewritten as decisions, §8 added),
`doc/rfc/README.md` (the index row), `doc/questions/A58` and `A54` (their `Owes:` lines), and this
file. Nothing under `crates/`.

**The finding worth a sentence**: a previous attempt at this round was cut off after one edit, and
its instruction to this one said the draft's quotation of `A58` was wrong — that the owner had
written "One amendment, not two — …" themselves. `git log -p` on `A58` shows why the instruction
and the file disagree: the owner's own session rewrote `A58` in 1ea10797, keeping the four words
"Q58 agree with recommendation" as the owner's and moving "one amendment, not two" into a
`Reading:` on the round's account, and `doc/questions/README.md` now names that file's first draft
as the forgery the convention prevents. The draft's attribution matched the file at HEAD, so it was
kept — the amendment quotes the four words and says the one-ruling reading was the round's
summary, ratified — and what was actually wrong in the draft was a claim, not a quotation: it
called the appended page "the one content stream in this program's output that is not a
producer's", which the review's table (`doc/reviews/984` §Q6) falsifies — variable-text
appearances and constructed `/AP`s are that already, each with a clause. The amendment now says
the true thing, which is `Q58`'s own: it is the one content stream this program writes that *no
clause specifies*.

**The amendment**, in `CLAUDE.md`: a page composed solely of content the document already holds
is on the near side of the "does the operation invent marks?" line; an archival conversion may
append one to carry what its target will not otherwise admit; the Level A structure entries such a
page owes are inside the same permission, one ruling; the watermark stays on the far side; and the
history sentence reads "amended three times — each by argument rather than by attrition". ADR
1014 carries the argument — what the exclusion was for, why fixed content with a verifiable source
is not composition in that sense, where the line runs in code against the review's table, and the
seven things an appending remedy owes before it is built.

**RFC 0007** moves from `proposed` to `accepted`. Each of the owner's seven answers is folded
into the section it decides, verbatim where the owner gave words (`A54`'s doubt, `A60`'s "don't
buy") and as the `Q` file's adopted sentences where the owner agreed to a recommendation; §7 keeps
the questions with the answers under them; §8 records that session 992 is building the format, the
sites and the departures this batch, and that later converter rounds owe the executor, the `derive`
guardrails, the single-alternative `on-failure`, the two switches, append-as-pages under the
amendment, and the catalogue. `A58`'s `Owes:` is `none`; `A54`'s loses its clause about this
RFC's ratification ADR and keeps the executor.

Sweeps: `--bin pointers` and `--bin quotations` before and after are in the round's report;
`cargo test -p conformance` ran at the end.
