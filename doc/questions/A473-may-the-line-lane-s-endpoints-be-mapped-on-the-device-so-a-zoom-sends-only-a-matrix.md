Status: complete
Given: 2026-10-19, in conversation — transcribed by the round
Owes: the build — the line lane's endpoints written once in page space and mapped by `line.wgsl`'s view uniform, so a zoom sends only a matrix; held to the line lane's existing bound against the processor and to a one-level tolerance between the kept and fresh device frames across adapters, in its own ADR with the instrument that shows which pages moved and by how much (acting round)

> I agree with both recommendations, please answer them.

Reading: the recommendation adopted is the question's own, quoted and
attributed: *"Accept it, held to the line lane's existing bound against the
processor and to a one-level tolerance between the kept and the fresh device
frames across adapters, as the compute lane is."*

What the answer settles is the owner's ask of 2026-10-19 — *"keep the page's
intermediate representation resident on the GPU and have a compute stage
re-rasterise it at the new transform, so a zoom sends only a matrix"* — for
the lane that carried 95 % of the witness's zoom bytes: lines are written once
in page space and the shader maps them by the view uniform. The compute lane's
fills already cross this same line under WGSL section 15.7.5 with the same
one-level tolerance (ADR 1420); the line lane joins that class, the processor
stays the oracle it was held to by a bound and never by bytes, and ADR 1877's
kept-frame-equals-fresh holds by construction. The instrument the discipline
owes for the step — its own ADR, and an arm printing which pages moved and by
how much — is in the owed build, and the witness's step goes from 287–301 ms
toward its replay's 127–141 ms once `doc/todo/75` item 3 builds the op list
once.