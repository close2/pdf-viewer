Status: complete
Given: 2026-09-28, in conversation — transcribed by the round
Owes: the two curves implemented of the tree's own, as private modules in `pdf-signature` behind the named-curve seam — brainpoolP512r1 over the reviewed `crypto-bigint` floor with RFC 5639 section 3.7's parameters, Ed448 with RFC 8032's field, curve and verification equation over the SHAKE256 already in `cms::Digest` — positive vectors from the RFCs' own suites and negatives derived from them, fuzzed with the module's existing targets, no crypto re-exported, and each module's ADR recording the swap condition: the day a stable, reviewed crate covers the curve, the swap is decided on `doc/stack.md`'s terms (acting round)

> Is my understanding for Q170 correct:
> Does the following sound reasonable (and does it indicate, that I have understood the question correctly):  the algorihtms belong into a crypto library.  We can implement without much effort implement it ourself.  We purposefully do not advertise it.  It is only a workaround in our code, until a "real"  crypto library implements it.  This also means, that we should implement it in a way, so that we can easily switch to a real crypto library later.  If this "sounds good".  Then yes.  That's what I want

Reading: the understanding is right in every part that matters, and this answer records the three
refinements given with the confirmation it asked for. **The missing set is two curves, not the
four the `Q` file first carried**: the `bp256`/`bp384` crates landed on their stable 0.14.0 line
since the 2026-08-23 measurement and `ecdsa.rs` already computes five of Table 3's six — so what
is left is brainpoolP512r1, for which no crate exists at all, and Ed448, for which no stable line
carries a signature scheme; the `Q` file beside this one is corrected accordingly, and the
`Owes` line is its scope. **"Without much effort" is the one part to read as a bound rather than
an afternoon**: brainpoolP512r1 needs field arithmetic for its 512-bit prime, and Ed448 a second
field, curve and equation — bounded, gated by the RFCs' own vectors, but real cryptographic work.
**The stopgap framing holds exactly as written for Ed448** — a reviewed crate is on the horizon,
and the swap is module for module — and for brainpoolP512r1 it holds until somebody publishes
one, which nobody has; the switchable shape is the named-curve seam, and the tree's own precedent
is the swap already made in the other direction: `bp256`/`bp384` were taken as crates the day
they were real. **Not advertising it** is the standing shape of every curve module already —
private to `pdf-signature`, no crypto re-exported, the public surface the signature questions.