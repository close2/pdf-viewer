# Q98 — How does §12.7.6.2's submit-form action reach the network?

Source: round 1193, making §12.7.6.2's ledger note exact.

## Where the clause stands in this tree

> Upon invocation of a submit-form action, an interactive PDF processor shall transmit the names
> and values of selected interactive form fields to a specified uniform resource locator (URL)

Every part of that sentence except the verb is a function of the document and of what a person
has entered, and all of it is built: `pdf_model::submission::compose` produces the method, the
URL, the media type and the body, in four formats, with every selection rule Tables 239 and 240
state applied. `viewer_core::interact` hands the composed request over as `Event::Submit` and
`viewer_host::policy::may_submit` is the single place the answer is decided — the shape
`CLAUDE.md`'s four restriction levels attach to (ADR 1062).

**The verb is what is missing, and only the verb.** `viewer-host` has no HTTP client, and the
platform handler it does have cannot stand in: `xdg-open` opens a URL with a GET and has no way
to carry an entity body, so a POST with `application/x-www-form-urlencoded`, `application/fdf`,
`application/xfdf` or `multipart/form-data` is not a thing it can be asked for.

Adding a network dependency is not a decision a round takes, which is why this is a question.

## The three options

**1. A reviewed HTTP client in `viewer-host`, behind the four levels.** `ureq` (blocking, no async
runtime — which `CLAUDE.md` principle 2 asks for) or `reqwest`. The request never touches the
sandboxed renderer: `viewer-host` is the unconfined side, and `pdf-model` and the confined worker
go on having no network at all, so principle 3's boundary is unmoved. `may_submit` becomes the
place the level is read — `off`, `on`, *ask*, *warn* — and a person is shown the URL and the
format before anything leaves the machine. Cost: a TLS stack and its transitive dependencies on
the host side, and a new class of thing this program does at a document's request.

**2. Write the composed request to a file, and let a person submit it.** No dependency at all.
The host saves the body, the method, the URL and the media type somewhere the person chooses and
says so; whoever wants to send it uses `curl`, a browser or a colleague's form. This is honest,
it is testable, and it keeps the program's network surface at zero — but it is not what the clause
says a processor does, so the row stays `partial` for ever and says why.

**3. Refuse by name, permanently.** The shape clause 13 is refused in: a decided exclusion with a
reason written down, `Action::Refused` carrying the sentence, and the row moved to a status that
records a choice rather than a gap. This is where the row stood before session 1047 and it is the
only option that ends the question rather than deferring it.

## Recommendation

**Option 1, with the default level `ask`.** The composition is finished and correct; refusing the
last step leaves a feature that is complete except for the thing it exists to do, and option 2
pays most of option 1's user-facing cost (a dialogue, a URL, a decision) while delivering none of
its usefulness. The security argument that made option 3 right in session 373 was about a
*capability this program had no safe place to put* — and it now has one: `viewer-host` is
unconfined by design, the renderer is not, and the policy seam is already built and already
asked. A document must not be able to submit silently, which is exactly what the four levels are
for, and `ask` is the level that makes the person the one who decides.

If option 1 is chosen, two things belong in the same decision: which client (`ureq` is the
smaller and needs no async runtime), and whether the level is per-document or global — both are
`doc/todo/38`'s existing surface rather than new machinery.

If the answer is option 2 or 3, say which, and the row records it as a decision; either is better
than the present state, where the note has to explain that nothing is missing but a dependency
nobody has ruled on.
