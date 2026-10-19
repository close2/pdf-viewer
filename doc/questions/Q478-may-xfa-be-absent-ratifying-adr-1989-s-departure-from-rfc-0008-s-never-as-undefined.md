# Q478 — May `xfa` be absent, ratifying ADR 1989's departure from RFC 0008's "never as `undefined`"?

Source: ADR 1989 (session 1576), which built the reading and left it on the owner's word — the
round's brief of 2026-10-19 lists "ADR 1989's departure" among the things that wait.
Status: **open** — answered when `A478-…` exists beside this file.

## Why it needs the owner

RFC 0008 §4.3 — ratified by `A193` — says an excluded call fails *"never as `undefined` and
never in silence"*, and its table lists `XFA`, `xfa`, `dynamicXFAForm`. ADR 1989 superseded that
row **for `xfa` alone**: a script's read finds the name not there (`typeof xfa` answers
`undefined`) and a use throws `ReferenceError`. A round may not widen a ratified amendment by
argument alone (the `Q65` precedent), so the departure waits here.

The argument is Adobe's own reference at §K.1's and §K.2's side. This processor takes §K.1's
permission not to implement XFA, and what it shows and fills is the AcroForm, which §K.2 makes
the form a script acts on in a hybrid file — an Acrobat form. The *JavaScript for Acrobat API
Reference*'s own XFA page says "The xfa object is undefined in an Acrobat form", and its one
example branches on exactly that: `if ( typeof xfa == "object" )` for an XML form, `else`
"This is an Acrobat form." A refusal (`NotAllowedError`) would tell the script it stands in an
XML form whose object it may not reach — the one thing not true of it here — and sends the
reference's own branch for this case unasked. `xfa` is not an excluded call failing: it is a
name the surface the reference describes does not have.

Nothing is silent. A document carrying an XFA resource, or stating `/NeedsRendering`, is said
once where the viewer opens it (ADR 1932); and a script that *uses* the name rather than asking
about it meets the `ReferenceError` an Acrobat form would give it, which the outcome reports as
any uncaught throw.

## What the tree does meanwhile

Built and held: `xfa` is in `surface::ABSENT`, `typeof` answers `undefined`, a use throws, and
no refusal is recorded — planted back into `EXCLUDED`, the test fails
(`crates/pdf-script/tests/fixtures.rs`; the script corpus walks in `doc/history/1576`).

## Recommendation

**Ratify it.** The reference's own sentence is the answer for the form this processor actually
shows, and the ratified rule binds excluded *calls*, which `xfa` — a name the Acrobat-form
surface does not have — is not. `XFA` and `this.dynamicXFAForm` stay refused exactly as RFC 0008
has them, so the exclusion's fence loses nothing.