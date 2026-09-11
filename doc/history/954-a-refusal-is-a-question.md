# 954 — A refusal was modelled as a verdict, and it is a question

Date: 2026-09-11. ADR: 0954. RFC: 0007 (`proposed`). Questions: `Q54`–`Q60`.
Files: `doc/rfc/0007-*.md` (new), `doc/adr/0954-*.md` (new), `doc/questions/Q54`–`Q60` (new),
`doc/pdf-a-conversion-limits.md` §3.1, §10.1.

The owner, on this tool being used in automatic environments: *a refusal would mean that a human
has to intervene.*

Read against ADR 0947 that is a design finding rather than a feature request. The converter was
never saying *this cannot be done*; it was saying *I will not do this without being told to*, and
then providing nowhere to be told. For one person converting one document those are
indistinguishable, which is why four sessions built on the first reading. For a queue they are
opposites.

**And `Decision` held a false pair.** Between "do it" and "stop" it offered a loss the user
authorises and a refusal — which made a whole class unthinkable: the answers where information
neither survives in place nor is lost but **moves** somewhere the target admits. Sorting by what
happens to the *content* gives `stop`, `discard`, `preserve`, `derive`, and the sort is what makes
a configuration legible to whoever writes it. One consequence nobody had seen: a signature's
mitigation is `preserve`, because what a signature tells a reader is a statement — who, when,
whether it verified — and that can go on a page even though the cryptography cannot survive.

**Three corrections came from the owner while the RFC was being written**, and each made it
simpler:

- a remedy belongs to a site **and a target**, and the first table said PDF/A-2 had no remedy for an
  embedded file when appending it as pages is available. The reason generalises: **what the six
  targets differ about is what may be *attached*, not what may be a page**;
- PDF/A-3 admits *all* attachments, so it is a wider permission rather than a narrower one — which
  means **a departure can be narrower than any target**, and that is the argument for departures
  rather than a caveat about them;
- and a profile answers refusals and speaks nowhere else, so `as-if-printed` is not incoherent with
  Level A. **Every answer a profile may give leaves the file conforming**, so no profile can make a
  conversion fail its target.

The third correction is the one worth carrying: the owner's *this is a replacement for a previous
paper archive* is the whole motivation for `as-if-printed`, and it composes with a target rather
than competing with it.
