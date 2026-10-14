Status: complete
Given: 2026-09-23, in conversation — transcribed by the round
Owes: the exception written into `CLAUDE.md` principle 3 — TLS for a transmission a person allowed, `ring` in the unconfined host, revisited when a pure-Rust provider is stable — and, with it, the validation the same answer asks for: the URL and above all the hostname it names, being document-derived input that enters the TLS stack, checked before anything is passed — a length cap, a character-set check, a refusal by name where either fails, rudimentary rather than exact and documented where it is performed (acting round)

> Are we sure, that no input touches the ring library?  isn't for instance the hostname passed to the library?  I am still for ring, but if this is the case, we should add some validation code, which makes sure, that only valid input is passed to ring.  This doesn't need to be very precise, but we should at least perform some rudimentary checks.

Reading: option 1, as the `Q` file recommends — `ring` in the unconfined host, a named exception in
principle 3 until a pure-Rust provider is stable — and the owner's own condition on it, with the
owner's question answered from the code before the answer was written: **the hostname is.**
`pdf_model::submission::compose` passes Table 239's `/F` through unchanged (`action.url`), and
`viewer_host::submit::transmit` sends it "to the URL exactly as composed"; the scheme gate
aside, ureq then hands the URL's hostname to `rustls` — for SNI encoding and for
`rustls-webpki`'s certificate-name verification — so a string a document chose is inside the
TLS stack. What still does not reach `ring` itself is any document-derived data: `ring`'s own
inputs are the server's chosen handshake bytes — keys, signatures, the transcript — which is
ADR 1291 §4's ground and stands. The validation is therefore owed at the seam the scheme check
already lives in, `transmit`, before the request is built: a hostname past a bound or holding
characters a name cannot is refused by name — rudimentary rather than exact, in the owner's
own words, with the checks and their bounds written where they run.