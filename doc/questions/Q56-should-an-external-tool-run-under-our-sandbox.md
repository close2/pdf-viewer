# Q56 — Should an external tool run under this project's sandbox where it can?

Source: `doc/rfc/0007` §7 question 3, raised by session 954.

## Why it needs the owner

`CLAUDE.md` principle 3 confines this project's renderer under seccomp-BPF and Landlock because it
processes untrusted input. An external remedy tool processes the same untrusted input — an operator
running `soffice` over a queue of documents from the public has made a security decision whether
they realise it or not.

The trouble is that most such tools **will not run** under a profile built for this tree's own
worker. Offering confinement for the few that would invites the belief that it is offered for all,
which is the more dangerous state: a partial guarantee read as a whole one.

## What the tree does meanwhile

No external tool is invoked. `crates/pdf-sandbox` confines this project's own decoders and nothing
else.

## Recommendation

Do not offer it in the first version, and **say so where an operator configures a tool** rather
than in a security document nobody opens. The honest sentence is short: *this runs a program you
chose, on a document you did not write.* Revisit if a real deployment asks for it, with that
deployment's tool in front of us — a confinement profile written against no particular program is
a guess.
