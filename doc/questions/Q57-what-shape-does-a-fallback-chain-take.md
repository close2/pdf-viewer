# Q57 — What shape does a remedy's fallback take when it fails?

Source: `doc/rfc/0007` §7 question 4, raised by session 954.
Status: **open** — answered when `A57-what-shape-does-a-fallback-chain-take.md` exists beside this
file.

## Why it needs the owner

An external tool can fail — a non-zero exit, a timeout, output larger than its bound, or output
that is not what `expects` declared. Something has to happen next, and the configuration has to say
what.

`on-failure = "discard"` picks one alternative and is trivial to reason about from a configuration
file. A **list** — try to derive, else preserve, else discard, else stop — is more expressive and
much harder to read: a reviewer looking at a fleet's configuration has to simulate it to know what
a document will actually get, and the whole point of this feature is that a queue's behaviour is
legible in advance.

## What the tree does meanwhile

Nothing fails, because nothing is invoked.

## Recommendation

One alternative, not a chain, defaulting to `stop`. If a real configuration turns out to want two
steps, that is evidence for a chain and can be added; adding expressiveness later is cheap, and
taking it away once somebody has written a file against it is not.
