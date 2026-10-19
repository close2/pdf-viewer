Status: complete
Given: 2026-10-19, in conversation — transcribed by the round
Owes: none — ADR 1989's build, its tests and the state-of-play sentence are already in the tree; this answer is the ratification the departure waited for

> I agree with your recommendation for Q478.  Please answer it.  I will think about Q287

Reading: the recommendation adopted is the question's own, quoted and attributed: *"Ratify
it."* ADR 1989's departure stands — a script's read of `xfa` finds the name not there, because
the form a script acts on here is the Acrobat form this processor shows and fills, and Adobe's
own reference answers for exactly that case: "The xfa object is undefined in an Acrobat form."

What the answer settles: RFC 0008 §4.3's *"never as `undefined`"* binds excluded **calls**, and
`xfa` is not one failing — it is a name the surface the reference describes does not have, with
nothing silent beside it (a document carrying an XFA resource or `/NeedsRendering` is said once
where the viewer opens it, ADR 1932; a script that uses the name meets the `ReferenceError` an
Acrobat form would give it). `XFA` and `this.dynamicXFAForm` stay refused exactly as the RFC has
them, so the exclusion's fence loses nothing. The supersession of the RFC's row for `xfa` alone
is ratified as ADR 1989 wrote it.