Status: complete
Given: 2026-10-05, in conversation — transcribed by the round
Owes: the definition written beside A170's swap condition, where ADR 1386 item 3 records it and the `Q` file's judged packages are named: **reviewed means stable, and from the supplier the tree already trusts** — under which all eight packages judged so far are "no", the tree's own `ed448.rs` stays, and the trigger is RustCrypto's `ed448-goldilocks` reaching a stable line that carries the signature scheme, at which the swap is decided on `doc/stack.md`'s terms without re-asking (acting round)

> I agree with all your recommendations.

Reading: **No — `ed448-goldilocks-plus` does not count as reviewed**, and no package the `Q`
file judged does. A package whose own README says, in capitals, that the code has not been audited
or reviewed cannot be "reviewed" under any reading, and a one-person fork is not the supplier
whose lineage the tree already trusts — RustCrypto's, shared with the ciphers and digests already
in the tree, which is what made every other curve crate acceptable. The `Q` file offered a weaker
reading of "reviewed" and this answer takes it as the standing definition, which settles all eight
judged packages at once and makes the future trigger precise: RustCrypto's own stable line, no
re-asking. The one nuance is kept with it: `frost-ed448`'s NCC audit exists, and it covered the
curve operations of `ed448-goldilocks` 0.9.0 — but that package is not the swap for a different
reason, its API refusing the identity and torsion points that RFC 8032 section 5.2.7's cofactored
equation exists to accept, which the tree's `ed448.rs` takes on purpose.