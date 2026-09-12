Status: complete
Given: 2026-09-12, in conversation — transcribed by the round
Owes: the single-alternative `on-failure` design in RFC 0007's implementation (acting round)

> Q57 agree with recommendation

Reading: as written in the `Q` file — `on-failure` picks one alternative, not a chain, and
defaults to `stop`. A chain is added only if a real configuration turns out to want two steps,
which is evidence rather than a preference: adding expressiveness later is cheap in a way that
taking it away, once somebody has written a file against it, is not.
