# 955 — Every requirement is a departure candidate, and a profile answers refusals only

Date: 2026-09-11. ADR: none of its own — the round's argument went into RFC 0007.
Files: `doc/rfc/0007-a-refusal-is-a-question-somebody-can-answer-in-advance.md` §4.7.5, §5a.

The owner, on the one departure the previous round had worked through: *note, that this we have
just discussed this single possible exclusion of the spec. there are probably a lot others where
different ways of "ignoring" the spec make sense. we need to think in every case, what could make
sense.*

So the round stopped treating the XML-attachment departure as a feature and started treating it as
**one instance of a question every requirement has to be asked.** §4.7.5 is the method, and the
useful half is that **one third of the sorting is free**: a requirement that does not bind all six
targets is one some part or flavour of ISO 19005 already relaxes, and the census can compute that
set without anybody reading a clause. What it cannot compute is the other two kinds — departing
still leaves an archive (a judgement, argued per requirement) and departing defeats what the format
is for (a refusal that stays). The second axis is orthogonal and was the owner's earlier
correction: a departure can be **narrowed** — "XML and only XML" — and narrowing is what makes one
safe rather than what qualifies it.

**§5a is the part that needed correcting twice, both times by the owner, and both corrections made
it smaller.** The round first let `as-if-printed` drift into something the converter *evaluates* —
a yardstick applied to content. The owner: *In this case I meant the "as-if-printed" more like
configuration choices regarding the refusals.* Then the round over-read it the other way, writing
that `as-if-printed` makes no sense for PDF/A-2a. The owner again:

> i wouldn't go as far as to say "as-if-printed makes no sense for PDF/A-2a". We are talking about
> refusals. The user wanted the output to have logical structure, but might think: this is a
> replacement for a previous paper archive. The logical structure is a nice to have, but I would
> rather lose information I wouldn't have had as paper archive anyway, if this means, that I can
> convert this input file.

That is the sentence the section is now built on. **A profile is answers to refusals and speaks
nowhere else**, which has a consequence worth more than the profiles themselves: every answer a
profile may give leaves the file conforming, so **no profile can make a conversion fail its
target**. A profile therefore composes with a target instead of competing with it, and the
strongest thing about the whole idea is that it needs no new mechanism at all.
